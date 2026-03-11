/// pcg-sync — PCG desktop file sync daemon
///
/// Syncs PCG data sources to a local folder, giving Finder/Nautilus access
/// without needing the full dashboard. New files dropped into the sync folder
/// are automatically uploaded to the PCG platform.
///
/// Config: ~/.pcg/sync.toml
/// State:  ~/.pcg/sync_state.json
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, multipart,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

// ─── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "pcg-sync", about = "PCG desktop file sync daemon")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the sync daemon (runs in foreground)
    Start,
    /// Print current config and sync status
    Status,
    /// Configure sync settings interactively
    Configure {
        #[arg(long, help = "PCG server URL (e.g. https://dashboard.powerclubglobal.com)")]
        server: Option<String>,
        #[arg(long, help = "Session token (copy from browser DevTools → Application → Cookies → session)")]
        token: Option<String>,
        #[arg(long, help = "Local folder to sync into (default: ~/PCG Files)")]
        folder: Option<String>,
        #[arg(long, help = "Organization ID to sync (leave blank to sync all)")]
        org_id: Option<String>,
    },
    /// Pull all files from server now (one-shot)
    Pull,
    /// Push all local files to server now (one-shot)
    Push,
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncConfig {
    /// PCG server base URL
    server_url: String,
    /// Session cookie value (from browser after login)
    session_token: String,
    /// Local folder to sync into
    sync_folder: PathBuf,
    /// Organisation ID filter (empty = all orgs)
    org_id: Option<String>,
    /// How often to poll the server (seconds)
    #[serde(default = "default_poll_interval")]
    poll_interval_secs: u64,
    /// Max file size to download automatically (bytes). Default 100MB.
    #[serde(default = "default_max_size")]
    max_auto_download_bytes: u64,
}

fn default_poll_interval() -> u64 { 60 }
fn default_max_size() -> u64 { 100 * 1024 * 1024 }

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pcg")
        .join("sync.toml")
}

fn state_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pcg")
        .join("sync_state.json")
}

fn load_config() -> Result<SyncConfig> {
    let path = config_path();
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Config not found at {}. Run: pcg-sync configure --server <url> --token <session>", path.display()))?;
    toml::from_str(&content).context("Invalid config file")
}

fn save_config(cfg: &SyncConfig) -> Result<()> {
    let path = config_path();
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, toml::to_string_pretty(cfg)?)?;
    println!("Config saved to {}", path.display());
    Ok(())
}

// ─── Sync State ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SyncState {
    /// Map of data source ID → local file path (relative to sync_folder)
    downloaded: HashMap<String, String>,
    /// Set of local file hashes that have been uploaded
    uploaded_hashes: HashSet<String>,
}

impl SyncState {
    fn load() -> Self {
        let path = state_path();
        if let Ok(content) = fs::read_to_string(&path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) -> Result<()> {
        let path = state_path();
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

// ─── API Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct DataSource {
    id: String,
    title: String,
    source_type: String,
    data_type: Option<String>,
    file_type: Option<String>,
    file_size: Option<i64>,
    #[serde(default)]
    metadata: String,
}

// ─── HTTP Client ──────────────────────────────────────────────────────────────

fn build_client(token: &str) -> Result<Client> {
    let mut headers = HeaderMap::new();
    let cookie = format!("session={}", token);
    headers.insert(header::COOKIE, HeaderValue::from_str(&cookie)?);
    Ok(Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .build()?)
}

async fn fetch_data_sources(client: &Client, cfg: &SyncConfig) -> Result<Vec<DataSource>> {
    let url = if let Some(ref org_id) = cfg.org_id {
        format!("{}/api/organizations/{}/data-sources", cfg.server_url, org_id)
    } else {
        format!("{}/api/data-sources", cfg.server_url)
    };

    let resp = client.get(&url).send().await?.error_for_status()?;
    let api: ApiResponse<Vec<DataSource>> = resp.json().await?;
    Ok(api.data)
}

/// Sanitize a filename to be filesystem-safe
fn safe_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || ".-_ ()[]{}".contains(c) { c } else { '_' })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Determine local path for a data source, preserving folder structure
fn local_path_for(source: &DataSource, sync_folder: &Path) -> PathBuf {
    let meta: serde_json::Value = serde_json::from_str(&source.metadata).unwrap_or_default();

    // Try to reconstruct folder context from Dropbox path
    let folder_context = meta.get("folder_context")
        .and_then(|v| v.as_str())
        .or_else(|| {
            meta.get("dropbox_path")
                .and_then(|v| v.as_str())
                .map(|p| p.rsplit('/').nth(1).unwrap_or(""))
        })
        .unwrap_or("");

    let ext = source.file_type.as_deref().unwrap_or("");
    let base_name = if ext.is_empty() || source.title.ends_with(&format!(".{}", ext)) {
        safe_filename(&source.title)
    } else {
        format!("{}.{}", safe_filename(&source.title), ext)
    };

    // Build path: sync_folder / Section / Client / filename
    if folder_context.is_empty() {
        sync_folder.join(&base_name)
    } else {
        let parts: Vec<&str> = folder_context.split(" > ").collect();
        let mut path = sync_folder.to_path_buf();
        for part in &parts {
            path = path.join(safe_filename(part));
        }
        path.join(&base_name)
    }
}

/// Check if a data source has a locally downloadable file
fn is_downloadable(source: &DataSource) -> bool {
    if source.source_type == "text" { return true; }
    let meta: serde_json::Value = serde_json::from_str(&source.metadata).unwrap_or_default();
    meta.get("file_path").is_some()
}

// ─── Pull (server → local) ────────────────────────────────────────────────────

async fn pull_all(client: &Client, cfg: &SyncConfig, state: &mut SyncState) -> Result<()> {
    info!("Fetching data sources from server...");
    let sources = fetch_data_sources(client, cfg).await?;
    info!("Found {} data sources", sources.len());

    let mut downloaded = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for source in &sources {
        // Skip if already downloaded
        if state.downloaded.contains_key(&source.id) {
            skipped += 1;
            continue;
        }

        // Skip if not locally available
        if !is_downloadable(source) {
            continue;
        }

        // Skip if too large
        if let Some(size) = source.file_size {
            if size > 0 && size as u64 > cfg.max_auto_download_bytes {
                warn!("Skipping {} ({} bytes — exceeds {} limit)", source.title, size, cfg.max_auto_download_bytes);
                continue;
            }
        }

        let local_path = local_path_for(source, &cfg.sync_folder);

        // Skip if file already exists at path
        if local_path.exists() {
            state.downloaded.insert(source.id.clone(), local_path.to_string_lossy().into_owned());
            continue;
        }

        // Download
        let url = format!("{}/api/data-sources/{}/download", cfg.server_url, source.id);
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.bytes().await {
                    Ok(bytes) => {
                        if let Some(parent) = local_path.parent() {
                            if let Err(e) = fs::create_dir_all(parent) {
                                error!("Failed to create directory {}: {}", parent.display(), e);
                                errors += 1;
                                continue;
                            }
                        }
                        if let Err(e) = fs::write(&local_path, &bytes) {
                            error!("Failed to write {}: {}", local_path.display(), e);
                            errors += 1;
                        } else {
                            info!("Downloaded: {} → {}", source.title, local_path.display());
                            state.downloaded.insert(source.id.clone(), local_path.to_string_lossy().into_owned());
                            downloaded += 1;
                        }
                    }
                    Err(e) => {
                        error!("Failed to read response for {}: {}", source.title, e);
                        errors += 1;
                    }
                }
            }
            Ok(resp) => {
                // 404 means no local file on server (Dropbox metadata only) — mark as known
                if resp.status().as_u16() == 404 {
                    state.downloaded.insert(source.id.clone(), "cloud-only".to_string());
                }
            }
            Err(e) => {
                error!("Failed to download {}: {}", source.title, e);
                errors += 1;
            }
        }
    }

    state.save()?;
    info!("Pull complete: {} downloaded, {} already synced, {} errors", downloaded, skipped, errors);
    Ok(())
}

// ─── Push (local → server) ────────────────────────────────────────────────────

fn file_hash(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(hex::encode(hasher.finalize()))
}

async fn push_file(client: &Client, cfg: &SyncConfig, path: &Path, state: &mut SyncState) -> Result<()> {
    let hash = file_hash(path).context("Failed to hash file")?;
    if state.uploaded_hashes.contains(&hash) {
        return Ok(()); // already uploaded
    }

    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let bytes = fs::read(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();

    // Determine org context from path
    let relative = path.strip_prefix(&cfg.sync_folder).unwrap_or(path);
    let _parts: Vec<&str> = relative.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    let form = multipart::Form::new()
        .text("title", filename.clone())
        .text("source_type", "upload")
        .text("data_type", "document")
        .part("file", multipart::Part::bytes(bytes).file_name(filename).mime_str(&mime)?);

    let form = if let Some(ref org_id) = cfg.org_id {
        form.text("organization_id", org_id.clone())
    } else { form };

    let url = format!("{}/api/data-sources/upload", cfg.server_url);
    let resp = client.post(&url).multipart(form).send().await?;

    if resp.status().is_success() {
        info!("Uploaded: {}", path.display());
        state.uploaded_hashes.insert(hash);
        state.save()?;
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        error!("Upload failed for {}: {} — {}", path.display(), status, body);
    }

    Ok(())
}

async fn push_all(client: &Client, cfg: &SyncConfig, state: &mut SyncState) -> Result<()> {
    info!("Scanning local folder for new files to upload...");
    let sync_folder = cfg.sync_folder.clone();

    if !sync_folder.exists() {
        fs::create_dir_all(&sync_folder)?;
    }

    // Known downloaded files — don't re-upload them
    let known_paths: HashSet<String> = state.downloaded.values()
        .filter(|p| *p != "cloud-only")
        .cloned()
        .collect();

    let mut uploaded = 0;
    let mut skipped = 0;

    for entry in walkdir_files(&sync_folder) {
        let path_str = entry.to_string_lossy().into_owned();
        if known_paths.contains(&path_str) {
            skipped += 1;
            continue;
        }
        if let Err(e) = push_file(client, cfg, &entry, state).await {
            error!("Failed to push {}: {}", entry.display(), e);
        } else {
            uploaded += 1;
        }
    }

    info!("Push complete: {} uploaded, {} already known", uploaded, skipped);
    Ok(())
}

fn walkdir_files(dir: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            !name.starts_with('.') && !name.starts_with("._")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

// ─── Daemon (watch + poll loop) ───────────────────────────────────────────────

async fn run_daemon(cfg: SyncConfig) -> Result<()> {
    info!("Starting PCG sync daemon");
    info!("  Server:      {}", cfg.server_url);
    info!("  Sync folder: {}", cfg.sync_folder.display());
    if let Some(ref org) = cfg.org_id {
        info!("  Org filter:  {}", org);
    }
    info!("  Poll:        every {}s", cfg.poll_interval_secs);

    fs::create_dir_all(&cfg.sync_folder)?;
    println!("📁 Sync folder ready: {}", cfg.sync_folder.display());
    println!("   Open this folder in Finder/Nautilus to access your PCG files.");
    println!("   Drop new files here — they'll upload automatically.\n");

    let client = build_client(&cfg.session_token)?;
    let state = Arc::new(Mutex::new(SyncState::load()));

    // Initial pull
    {
        let mut st = state.lock().await;
        if let Err(e) = pull_all(&client, &cfg, &mut st).await {
            error!("Initial pull failed: {}", e);
        }
    }

    // Filesystem watcher for push
    let (tx, mut rx) = tokio::sync::mpsc::channel::<PathBuf>(64);
    let watch_folder = cfg.sync_folder.clone();
    let _watcher = {
        let tx = tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    for path in event.paths {
                        if path.is_file() {
                            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            if !name.starts_with('.') {
                                let _ = tx.blocking_send(path);
                            }
                        }
                    }
                }
            }
        })?;
        watcher.watch(&watch_folder, RecursiveMode::Recursive)?;
        watcher
    };

    // Poll timer
    let poll_interval = Duration::from_secs(cfg.poll_interval_secs);
    let mut poll_ticker = tokio::time::interval(poll_interval);
    poll_ticker.tick().await; // skip first immediate tick (already pulled)

    loop {
        tokio::select! {
            _ = poll_ticker.tick() => {
                info!("Poll: checking for new files on server...");
                let mut st = state.lock().await;
                if let Err(e) = pull_all(&client, &cfg, &mut st).await {
                    error!("Pull failed: {}", e);
                }
            }

            Some(path) = rx.recv() => {
                // Debounce: wait briefly for writes to complete
                tokio::time::sleep(Duration::from_millis(500)).await;
                // Drain any more pending events for the same file
                while rx.try_recv().is_ok() {}

                if path.exists() && path.is_file() {
                    let mut st = state.lock().await;
                    if let Err(e) = push_file(&client, &cfg, &path, &mut st).await {
                        error!("Push failed for {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
}

// ─── Status ───────────────────────────────────────────────────────────────────

fn print_status() {
    match load_config() {
        Ok(cfg) => {
            println!("PCG Sync Configuration:");
            println!("  Server:      {}", cfg.server_url);
            println!("  Sync folder: {}", cfg.sync_folder.display());
            println!("  Org filter:  {}", cfg.org_id.as_deref().unwrap_or("all"));
            println!("  Poll:        every {}s", cfg.poll_interval_secs);
            println!("  Max dl size: {} MB", cfg.max_auto_download_bytes / 1024 / 1024);

            let state = SyncState::load();
            let local_count = state.downloaded.values().filter(|p| *p != "cloud-only").count();
            let cloud_only = state.downloaded.values().filter(|p| *p == "cloud-only").count();
            let uploaded = state.uploaded_hashes.len();
            println!("\nSync State:");
            println!("  Local files: {}", local_count);
            println!("  Cloud-only:  {} (metadata only, no local file on server)", cloud_only);
            println!("  Uploaded:    {}", uploaded);

            if cfg.sync_folder.exists() {
                let file_count = walkdir_files(&cfg.sync_folder).len();
                println!("  Files in folder: {}", file_count);
            }
        }
        Err(e) => {
            eprintln!("No config found: {}", e);
            eprintln!("Run: pcg-sync configure --server <url> --token <session>");
        }
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("pcg_sync=info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Configure { server, token, folder, org_id } => {
            let sync_folder = folder
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join("PCG Files")
                });

            let server_url = server.unwrap_or_else(|| {
                eprint!("Server URL [https://dashboard.powerclubglobal.com]: ");
                let mut s = String::new();
                std::io::stdin().read_line(&mut s).ok();
                let s = s.trim().to_string();
                if s.is_empty() { "https://dashboard.powerclubglobal.com".to_string() } else { s }
            });

            let session_token = token.unwrap_or_else(|| {
                eprintln!("Get your session token from browser DevTools:");
                eprintln!("  1. Open the PCG dashboard and log in");
                eprintln!("  2. Open DevTools (F12) → Application → Cookies");
                eprintln!("  3. Copy the 'session' cookie value");
                eprint!("Session token: ");
                let mut s = String::new();
                std::io::stdin().read_line(&mut s).ok();
                s.trim().to_string()
            });

            let cfg = SyncConfig {
                server_url,
                session_token,
                sync_folder: sync_folder.clone(),
                org_id,
                poll_interval_secs: 60,
                max_auto_download_bytes: 100 * 1024 * 1024,
            };

            save_config(&cfg)?;
            println!("Sync folder: {}", sync_folder.display());
            println!("\nTo start syncing:");
            println!("  pcg-sync start");
            println!("\nTo install as a background service (Linux systemd):");
            println!("  See README for systemd unit file setup");
        }

        Command::Status => {
            print_status();
        }

        Command::Start => {
            let cfg = load_config()?;
            run_daemon(cfg).await?;
        }

        Command::Pull => {
            let cfg = load_config()?;
            let client = build_client(&cfg.session_token)?;
            let mut state = SyncState::load();
            pull_all(&client, &cfg, &mut state).await?;
        }

        Command::Push => {
            let cfg = load_config()?;
            let client = build_client(&cfg.session_token)?;
            let mut state = SyncState::load();
            push_all(&client, &cfg, &mut state).await?;
        }
    }

    Ok(())
}
