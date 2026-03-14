/// pcg-sync — PCG desktop file sync daemon
///
/// Syncs PCG data sources to a local folder, giving Finder/Nautilus access
/// without needing the full dashboard. New files dropped into the sync folder
/// are automatically uploaded to the PCG platform.
///
/// Enhanced with:
/// - Server-side sync folder hierarchy (org-scoped shared folders)
/// - Device registration & heartbeat
/// - Selective sync via folder subscriptions
/// - Conflict detection via hash comparison
/// - Sync state reporting back to server
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
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
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
        #[arg(long, help = "Device name (default: hostname)")]
        device_name: Option<String>,
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
    /// Device name for registration
    #[serde(default = "default_device_name")]
    device_name: String,
    /// Registered device ID (set after first registration)
    #[serde(default)]
    device_id: Option<String>,
}

fn default_poll_interval() -> u64 { 60 }
fn default_max_size() -> u64 { 100 * 1024 * 1024 }
fn default_device_name() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Unknown Device".to_string())
}

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
struct SyncStateLocal {
    /// Map of data source ID → local file path (relative to sync_folder)
    downloaded: HashMap<String, DownloadedEntry>,
    /// Set of local file hashes that have been uploaded
    uploaded_hashes: HashSet<String>,
    /// Map of file path → hash for conflict detection
    file_hashes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadedEntry {
    path: String,
    hash: Option<String>,
    size: Option<u64>,
    synced_at: Option<String>,
    folder_id: Option<String>,
}

impl SyncStateLocal {
    fn load() -> Self {
        let path = state_path();
        if let Ok(content) = fs::read_to_string(&path) {
            // Try new format first, fall back to old
            if let Ok(state) = serde_json::from_str::<Self>(&content) {
                return state;
            }
            // Migrate from old format (HashMap<String, String>)
            if let Ok(old) = serde_json::from_str::<OldSyncState>(&content) {
                let mut new = Self::default();
                for (id, path) in old.downloaded {
                    new.downloaded.insert(id, DownloadedEntry {
                        path,
                        hash: None,
                        size: None,
                        synced_at: None,
                        folder_id: None,
                    });
                }
                new.uploaded_hashes = old.uploaded_hashes;
                return new;
            }
        }
        Self::default()
    }

    fn save(&self) -> Result<()> {
        let path = state_path();
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct OldSyncState {
    downloaded: HashMap<String, String>,
    uploaded_hashes: HashSet<String>,
}

// ─── API Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize, Clone)]
struct DataSource {
    id: String,
    title: String,
    source_type: String,
    data_type: Option<String>,
    file_type: Option<String>,
    file_size: Option<i64>,
    file_size_bytes: Option<i64>,
    #[serde(default)]
    metadata: String,
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct SyncFolderRemote {
    id: String,
    name: String,
    path: String,
    auto_sync: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct SyncDeviceRemote {
    id: String,
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

async fn fetch_sync_folders(client: &Client, cfg: &SyncConfig) -> Result<Vec<SyncFolderRemote>> {
    let Some(ref org_id) = cfg.org_id else {
        return Ok(Vec::new());
    };
    let url = format!("{}/api/organizations/{}/sync/folders", cfg.server_url, org_id);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let api: ApiResponse<Vec<SyncFolderRemote>> = resp.json().await?;
            Ok(api.data)
        }
        _ => Ok(Vec::new()),
    }
}

async fn register_device(client: &Client, cfg: &mut SyncConfig) -> Result<String> {
    if let Some(ref id) = cfg.device_id {
        // Heartbeat existing device
        let url = format!("{}/api/sync/devices/{}/heartbeat", cfg.server_url, id);
        if client.post(&url).send().await.is_ok() {
            return Ok(id.clone());
        }
    }

    // Register new device
    let platform = std::env::consts::OS.to_string();
    let body = serde_json::json!({
        "user_id": "00000000-0000-0000-0000-000000000000",
        "organization_id": cfg.org_id.as_deref().unwrap_or(""),
        "device_name": cfg.device_name,
        "device_type": "desktop",
        "platform": platform,
        "sync_folder": cfg.sync_folder.to_string_lossy(),
    });

    let url = format!("{}/api/sync/devices/register", cfg.server_url);
    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let api: ApiResponse<SyncDeviceRemote> = resp.json().await?;
            cfg.device_id = Some(api.data.id.clone());
            save_config(cfg)?;
            info!("Device registered: {}", api.data.id);
            Ok(api.data.id)
        }
        Ok(resp) => {
            let status = resp.status();
            warn!("Device registration failed ({}), running in offline mode", status);
            Ok(String::new())
        }
        Err(e) => {
            warn!("Device registration failed: {}, running in offline mode", e);
            Ok(String::new())
        }
    }
}

async fn report_sync_state(
    client: &Client,
    cfg: &SyncConfig,
    file_path: &str,
    status: &str,
    hash: Option<&str>,
    size: Option<i64>,
    data_source_id: Option<&str>,
    direction: &str,
) {
    let Some(ref device_id) = cfg.device_id else { return };
    let url = format!("{}/api/sync/devices/{}/state", cfg.server_url, device_id);
    let body = serde_json::json!({
        "device_id": device_id,
        "file_path": file_path,
        "sync_status": status,
        "file_hash": hash,
        "file_size": size,
        "data_source_id": data_source_id,
        "sync_direction": direction,
    });

    if let Err(e) = client.post(&url).json(&body).send().await {
        warn!("Failed to report sync state: {}", e);
    }
}

/// Sanitize a filename to be filesystem-safe
fn safe_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || ".-_ ()[]{}".contains(c) { c } else { '_' })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Determine local path for a data source, using sync folder hierarchy
fn local_path_for(source: &DataSource, sync_folder: &Path, sync_folders: &[SyncFolderRemote]) -> PathBuf {
    let meta: serde_json::Value = serde_json::from_str(&source.metadata).unwrap_or_default();

    // Try to match to a sync folder first
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

    // If we have sync folders from server, use their paths for organization
    if !sync_folders.is_empty() && !folder_context.is_empty() {
        // Find matching sync folder by path prefix
        for sf in sync_folders {
            let sf_parts: Vec<&str> = sf.path.trim_start_matches('/').split('/').collect();
            let ctx_parts: Vec<&str> = folder_context.split(" > ").collect();
            if !sf_parts.is_empty() && !ctx_parts.is_empty() && sf_parts[0].eq_ignore_ascii_case(ctx_parts[0]) {
                let mut path = sync_folder.to_path_buf();
                for part in &ctx_parts {
                    path = path.join(safe_filename(part));
                }
                return path.join(&base_name);
            }
        }
    }

    // Fallback: use folder context from metadata
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

fn file_hash(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(hex::encode(hasher.finalize()))
}

// ─── Pull (server → local) ────────────────────────────────────────────────────

async fn pull_all(client: &Client, cfg: &SyncConfig, state: &mut SyncStateLocal) -> Result<()> {
    info!("Fetching data sources from server...");
    let sources = fetch_data_sources(client, cfg).await?;
    let sync_folders = fetch_sync_folders(client, cfg).await.unwrap_or_default();
    info!("Found {} data sources, {} sync folders", sources.len(), sync_folders.len());

    let mut downloaded = 0;
    let mut skipped = 0;
    let mut conflicts = 0;
    let mut errors = 0;

    for source in &sources {
        // Skip if not locally available
        if !is_downloadable(source) {
            continue;
        }

        let local_path = local_path_for(source, &cfg.sync_folder, &sync_folders);
        let relative_path = local_path.strip_prefix(&cfg.sync_folder)
            .unwrap_or(&local_path)
            .to_string_lossy()
            .to_string();

        // Check if already downloaded with same version
        if let Some(entry) = state.downloaded.get(&source.id) {
            // Check for conflict: local file modified since last sync
            if local_path.exists() {
                if let Some(ref stored_hash) = entry.hash {
                    if let Some(current_hash) = file_hash(&local_path) {
                        if &current_hash != stored_hash {
                            // Local file was modified — check if remote also changed
                            if source.updated_at.is_some() && entry.synced_at.is_some() {
                                warn!("CONFLICT: {} modified both locally and remotely", source.title);
                                report_sync_state(
                                    client, cfg, &relative_path, "conflict",
                                    Some(&current_hash), None, Some(&source.id), "pull",
                                ).await;
                                conflicts += 1;
                                continue;
                            }
                        }
                    }
                }
            }
            skipped += 1;
            continue;
        }

        // Skip if too large
        let file_size = source.file_size_bytes.or(source.file_size);
        if let Some(size) = file_size {
            if size > 0 && size as u64 > cfg.max_auto_download_bytes {
                warn!("Skipping {} ({} bytes — exceeds {} limit)", source.title, size, cfg.max_auto_download_bytes);
                continue;
            }
        }

        // Skip if file already exists at path (first-time setup)
        if local_path.exists() {
            let hash = file_hash(&local_path);
            state.downloaded.insert(source.id.clone(), DownloadedEntry {
                path: relative_path.clone(),
                hash: hash.clone(),
                size: fs::metadata(&local_path).ok().map(|m| m.len()),
                synced_at: Some(chrono::Utc::now().to_rfc3339()),
                folder_id: None,
            });
            report_sync_state(
                client, cfg, &relative_path, "synced",
                hash.as_deref(), file_size, Some(&source.id), "pull",
            ).await;
            continue;
        }

        // Download
        let url = format!("{}/api/data-sources/{}/download", cfg.server_url, source.id);

        report_sync_state(
            client, cfg, &relative_path, "pending",
            None, file_size, Some(&source.id), "pull",
        ).await;

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
                            report_sync_state(
                                client, cfg, &relative_path, "error",
                                None, file_size, Some(&source.id), "pull",
                            ).await;
                            errors += 1;
                        } else {
                            let hash = file_hash(&local_path);
                            let size = bytes.len() as u64;
                            info!("Downloaded: {} → {}", source.title, local_path.display());
                            state.downloaded.insert(source.id.clone(), DownloadedEntry {
                                path: relative_path.clone(),
                                hash: hash.clone(),
                                size: Some(size),
                                synced_at: Some(chrono::Utc::now().to_rfc3339()),
                                folder_id: None,
                            });
                            state.file_hashes.insert(relative_path.clone(), hash.clone().unwrap_or_default());
                            report_sync_state(
                                client, cfg, &relative_path, "synced",
                                hash.as_deref(), Some(size as i64), Some(&source.id), "pull",
                            ).await;
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
                if resp.status().as_u16() == 404 {
                    state.downloaded.insert(source.id.clone(), DownloadedEntry {
                        path: "cloud-only".to_string(),
                        hash: None,
                        size: None,
                        synced_at: None,
                        folder_id: None,
                    });
                }
            }
            Err(e) => {
                error!("Failed to download {}: {}", source.title, e);
                errors += 1;
            }
        }
    }

    state.save()?;
    info!(
        "Pull complete: {} downloaded, {} already synced, {} conflicts, {} errors",
        downloaded, skipped, conflicts, errors
    );
    Ok(())
}

// ─── Push (local → server) ────────────────────────────────────────────────────

async fn push_file(client: &Client, cfg: &SyncConfig, path: &Path, state: &mut SyncStateLocal) -> Result<()> {
    let hash = file_hash(path).context("Failed to hash file")?;
    if state.uploaded_hashes.contains(&hash) {
        return Ok(()); // already uploaded
    }

    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let relative_path = path.strip_prefix(&cfg.sync_folder)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    // Report pending
    report_sync_state(
        client, cfg, &relative_path, "pending",
        Some(&hash), None, None, "push",
    ).await;

    let bytes = fs::read(path)?;
    let size = bytes.len() as i64;
    let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();

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
        state.uploaded_hashes.insert(hash.clone());
        state.file_hashes.insert(relative_path.clone(), hash.clone());
        report_sync_state(
            client, cfg, &relative_path, "synced",
            Some(&hash), Some(size), None, "push",
        ).await;
        state.save()?;
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        error!("Upload failed for {}: {} — {}", path.display(), status, body);
        report_sync_state(
            client, cfg, &relative_path, "error",
            Some(&hash), Some(size), None, "push",
        ).await;
    }

    Ok(())
}

async fn push_all(client: &Client, cfg: &SyncConfig, state: &mut SyncStateLocal) -> Result<()> {
    info!("Scanning local folder for new files to upload...");
    let sync_folder = cfg.sync_folder.clone();

    if !sync_folder.exists() {
        fs::create_dir_all(&sync_folder)?;
    }

    // Known downloaded files — don't re-upload them
    let known_paths: HashSet<String> = state.downloaded.values()
        .filter(|e| e.path != "cloud-only")
        .map(|e| e.path.clone())
        .collect();

    let mut uploaded = 0;
    let mut skipped = 0;

    for entry in walkdir_files(&sync_folder) {
        let relative = entry.strip_prefix(&sync_folder)
            .unwrap_or(&entry)
            .to_string_lossy()
            .to_string();
        if known_paths.contains(&relative) {
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

async fn run_daemon(mut cfg: SyncConfig) -> Result<()> {
    info!("Starting PCG sync daemon");
    info!("  Server:      {}", cfg.server_url);
    info!("  Sync folder: {}", cfg.sync_folder.display());
    info!("  Device:      {}", cfg.device_name);
    if let Some(ref org) = cfg.org_id {
        info!("  Org filter:  {}", org);
    }
    info!("  Poll:        every {}s", cfg.poll_interval_secs);

    fs::create_dir_all(&cfg.sync_folder)?;
    println!("  Sync folder ready: {}", cfg.sync_folder.display());
    println!("   Open this folder in Finder/Nautilus to access your PCG files.");
    println!("   Drop new files here — they'll upload automatically.\n");

    let client = build_client(&cfg.session_token)?;

    // Register device with server
    let device_id = register_device(&client, &mut cfg).await?;
    if !device_id.is_empty() {
        info!("Device ID: {}", device_id);
    }

    let state = Arc::new(Mutex::new(SyncStateLocal::load()));

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

    // Heartbeat timer (every 60s)
    let mut heartbeat_ticker = tokio::time::interval(Duration::from_secs(60));
    heartbeat_ticker.tick().await;

    loop {
        tokio::select! {
            _ = poll_ticker.tick() => {
                info!("Poll: checking for new files on server...");
                let mut st = state.lock().await;
                if let Err(e) = pull_all(&client, &cfg, &mut st).await {
                    error!("Pull failed: {}", e);
                }
            }

            _ = heartbeat_ticker.tick() => {
                if let Some(ref id) = cfg.device_id {
                    let url = format!("{}/api/sync/devices/{}/heartbeat", cfg.server_url, id);
                    let _ = client.post(&url).send().await;
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
            println!("  Device:      {}", cfg.device_name);
            println!("  Device ID:   {}", cfg.device_id.as_deref().unwrap_or("not registered"));
            println!("  Org filter:  {}", cfg.org_id.as_deref().unwrap_or("all"));
            println!("  Poll:        every {}s", cfg.poll_interval_secs);
            println!("  Max dl size: {} MB", cfg.max_auto_download_bytes / 1024 / 1024);

            let state = SyncStateLocal::load();
            let local_count = state.downloaded.values().filter(|e| e.path != "cloud-only").count();
            let cloud_only = state.downloaded.values().filter(|e| e.path == "cloud-only").count();
            let uploaded = state.uploaded_hashes.len();
            let tracked_hashes = state.file_hashes.len();
            println!("\nSync State:");
            println!("  Local files:    {}", local_count);
            println!("  Cloud-only:     {} (metadata only)", cloud_only);
            println!("  Uploaded:       {}", uploaded);
            println!("  Tracked hashes: {}", tracked_hashes);

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
        Command::Configure { server, token, folder, org_id, device_name } => {
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
                device_name: device_name.unwrap_or_else(default_device_name),
                device_id: None,
            };

            save_config(&cfg)?;
            println!("Sync folder: {}", sync_folder.display());
            println!("\nTo start syncing:");
            println!("  pcg-sync start");
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
            let mut state = SyncStateLocal::load();
            pull_all(&client, &cfg, &mut state).await?;
        }

        Command::Push => {
            let cfg = load_config()?;
            let client = build_client(&cfg.session_token)?;
            let mut state = SyncStateLocal::load();
            push_all(&client, &cfg, &mut state).await?;
        }
    }

    Ok(())
}
