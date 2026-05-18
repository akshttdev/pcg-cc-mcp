//! Sovereign Stack Scraper Service
//!
//! Background service that copies Dropbox-synced data from C: drive to the
//! sovereign stack on E: drive. This creates the organization's canonical,
//! sovereign-controlled file hosting layer.
//!
//! Data flow: Dropbox Cloud -> C: (Dropbox desktop app) -> E: (sovereign stack)
//!
//! Environment variables:
//!   SOVEREIGN_STACK_ENABLED          - true/false (default: false)
//!   SOVEREIGN_STACK_ORG_NAME         - org directory name (default: "Sirak Studios")
//!   SOVEREIGN_STACK_ROOT             - root path (default: "E:/topos/sovereign_stack")
//!   SOVEREIGN_STACK_DROPBOX_ROOT     - Dropbox sync root on E: (default: "E:/topos/Sirak Studios Dropbox")
//!   SOVEREIGN_STACK_DROPBOX_PERSONAL - personal subdir (default: "Sirak Studios (sirak)")
//!   SOVEREIGN_STACK_DROPBOX_TEAM     - team subdir (default: "Sirak Studios Team")
//!   SOVEREIGN_STACK_SCAN_INTERVAL    - seconds between scans (default: 300)

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::time;
use tokio_util::sync::CancellationToken;

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct SovereignStackConfig {
    pub enabled: bool,
    pub org_name: String,
    pub root: PathBuf,
    pub dropbox_root: PathBuf,
    pub dropbox_personal: String,
    pub dropbox_team: String,
    pub scan_interval_secs: u64,
}

impl SovereignStackConfig {
    pub fn from_env() -> Result<Self> {
        let enabled = std::env::var("SOVEREIGN_STACK_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let org_name = std::env::var("SOVEREIGN_STACK_ORG_NAME")
            .unwrap_or_else(|_| "Sirak Studios".to_string());

        let root = PathBuf::from(
            std::env::var("SOVEREIGN_STACK_ROOT")
                .unwrap_or_else(|_| "E:/topos/sovereign_stack".to_string()),
        );

        let dropbox_root = PathBuf::from(
            std::env::var("SOVEREIGN_STACK_DROPBOX_ROOT")
                .unwrap_or_else(|_| "E:/topos/Sirak Studios Dropbox".to_string()),
        );

        let dropbox_personal = std::env::var("SOVEREIGN_STACK_DROPBOX_PERSONAL")
            .unwrap_or_else(|_| "Sirak Studios (sirak)".to_string());

        let dropbox_team = std::env::var("SOVEREIGN_STACK_DROPBOX_TEAM")
            .unwrap_or_else(|_| "Sirak Studios Team".to_string());

        let scan_interval_secs = std::env::var("SOVEREIGN_STACK_SCAN_INTERVAL")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .unwrap_or(300);

        Ok(Self {
            enabled,
            org_name,
            root,
            dropbox_root,
            dropbox_personal,
            dropbox_team,
            scan_interval_secs,
        })
    }

    /// Org root: E:/topos/sovereign_stack/Sirak Studios/
    pub fn org_root(&self) -> PathBuf {
        self.root.join(&self.org_name)
    }

    /// Personal storage: E:/topos/sovereign_stack/Personal/
    pub fn sovereign_personal(&self) -> PathBuf {
        self.root.join("Personal")
    }

    /// Organization root (Team data merges here): E:/topos/sovereign_stack/Sirak Studios/
    pub fn sovereign_org(&self) -> PathBuf {
        self.org_root()
    }

    /// Source Dropbox Personal on C:
    pub fn source_personal(&self) -> PathBuf {
        self.dropbox_root.join(&self.dropbox_personal)
    }

    /// Source Dropbox Team on C:
    pub fn source_team(&self) -> PathBuf {
        self.dropbox_root.join(&self.dropbox_team)
    }

    /// Manifest path: E:/topos/sovereign_stack/Sirak Studios/.sovereign/manifest.json
    pub fn manifest_path(&self) -> PathBuf {
        self.org_root().join(".sovereign").join("manifest.json")
    }
}

// ============================================================================
// Manifest
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub source_volume: String,
    pub copied_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub org_name: String,
    pub last_scan: Option<String>,
    pub total_files: u64,
    pub total_bytes: u64,
    pub files: HashMap<String, ManifestEntry>,
}

impl Manifest {
    fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(data) => match serde_json::from_str(&data) {
                Ok(manifest) => manifest,
                Err(e) => {
                    tracing::warn!(
                        "[SOVEREIGN_STACK] Failed to parse manifest at {:?}: {}; using default",
                        path,
                        e
                    );
                    Self::default()
                }
            },
            Err(e) => {
                // File not found is expected on first run — only warn on other errors
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(
                        "[SOVEREIGN_STACK] Failed to read manifest at {:?}: {}; using default",
                        path,
                        e
                    );
                }
                Self::default()
            }
        }
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create manifest dir: {:?}", parent))?;
        }
        let json = serde_json::to_string_pretty(self).context("Failed to serialize manifest")?;
        std::fs::write(path, json)
            .with_context(|| format!("Failed to write manifest: {:?}", path))?;
        Ok(())
    }
}

// ============================================================================
// Skip list
// ============================================================================

/// Names/suffixes to skip during scraping
fn should_skip(name: &str) -> bool {
    let lower = name.to_lowercase();

    // Hidden files
    if name.starts_with('.') {
        return true;
    }

    // System/cache files
    matches!(lower.as_str(), "desktop.ini" | "thumbs.db" | ".ds_store")
}

/// Directory names to skip
fn should_skip_dir(name: &str) -> bool {
    let lower = name.to_lowercase();

    // Hidden dirs
    if name.starts_with('.') {
        return true;
    }

    // Specific skip patterns
    if lower.ends_with(".lrdata")
        || lower.ends_with(".lrcat-data")
        || lower == ".dropbox.cache"
        || lower == "node_modules"
        || lower == ".git"
        || lower == "target"
        || lower == "__pycache__"
    {
        return true;
    }

    false
}

// ============================================================================
// Service
// ============================================================================

pub struct SovereignStackService {
    config: SovereignStackConfig,
    pool: Option<sqlx::SqlitePool>,
    org_id: String,
}

impl SovereignStackService {
    pub fn new(
        config: SovereignStackConfig,
        pool: Option<sqlx::SqlitePool>,
        org_id: String,
    ) -> Self {
        Self {
            config,
            pool,
            org_id,
        }
    }

    /// Start the background scraper loop.
    ///
    /// Accepts a `CancellationToken` for graceful shutdown — when the token is
    /// cancelled, the loop exits cleanly after the current tick completes.
    pub async fn start(&mut self, shutdown: CancellationToken) -> Result<()> {
        tracing::info!(
            "[SOVEREIGN_STACK] Starting scraper service (interval: {}s)",
            self.config.scan_interval_secs
        );
        tracing::info!(
            "[SOVEREIGN_STACK] Dropbox root: {:?}",
            self.config.dropbox_root
        );
        tracing::info!(
            "[SOVEREIGN_STACK] Sovereign root: {:?}",
            self.config.org_root()
        );

        // Create directory structure on startup
        self.ensure_directories().await?;

        let mut interval = time::interval(Duration::from_secs(self.config.scan_interval_secs));

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown.cancelled() => {
                    tracing::info!("[SOVEREIGN_STACK] Shutting down scraper service");
                    break;
                }
            }

            if let Err(e) = self.run_scan().await {
                tracing::error!("[SOVEREIGN_STACK] Scan failed: {}", e);
            }

            // Re-index cloud files after each scan so new Dropbox files appear in the Cloud Browser
            if let Some(ref pool) = self.pool {
                match crate::org_cloud_indexer::index_existing_data(pool, &self.org_id).await {
                    Ok(n) if n > 0 => {
                        tracing::info!("[SOVEREIGN_STACK] Re-indexed {} new cloud files", n)
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("[SOVEREIGN_STACK] Cloud re-index failed: {}", e),
                }
            }
        }

        Ok(())
    }

    /// Create the sovereign stack directory structure if missing
    async fn ensure_directories(&self) -> Result<()> {
        let dirs = [
            self.config.sovereign_personal(),
            self.config.sovereign_org(),
            self.config.org_root().join("Media Pipeline"),
            self.config.org_root().join("Data Sources"),
            self.config.org_root().join("Artifacts"),
            self.config.org_root().join(".sovereign"),
        ];

        for dir in &dirs {
            tokio::fs::create_dir_all(dir)
                .await
                .with_context(|| format!("Failed to create dir: {:?}", dir))?;
        }

        tracing::info!("[SOVEREIGN_STACK] Directory structure verified");
        Ok(())
    }

    /// Run a single scan cycle: walk Dropbox dirs on C:, copy new/modified to E:
    async fn run_scan(&self) -> Result<()> {
        let mut manifest = Manifest::load(&self.config.manifest_path());
        manifest.org_name = self.config.org_name.clone();

        let mut total_copied: u64 = 0;
        let mut total_skipped: u64 = 0;

        // Scan Personal Dropbox → sovereign_stack/Personal/
        let source_personal = self.config.source_personal();
        let dest_personal = self.config.sovereign_personal();
        if source_personal.exists() {
            let (copied, skipped) = self
                .scrape_directory(
                    &source_personal,
                    &dest_personal,
                    "sovereign_personal",
                    &mut manifest,
                )
                .await?;
            total_copied += copied;
            total_skipped += skipped;
        } else {
            tracing::debug!(
                "[SOVEREIGN_STACK] Personal Dropbox not found at {:?}",
                source_personal
            );
        }

        // Scan Team Dropbox → sovereign_stack/Sirak Studios/ (org root)
        let source_team = self.config.source_team();
        let dest_org = self.config.sovereign_org();
        if source_team.exists() {
            let (copied, skipped) = self
                .scrape_directory(&source_team, &dest_org, "sovereign_org", &mut manifest)
                .await?;
            total_copied += copied;
            total_skipped += skipped;
        } else {
            tracing::debug!(
                "[SOVEREIGN_STACK] Team Dropbox not found at {:?}",
                source_team
            );
        }

        // Update manifest
        manifest.last_scan = Some(chrono::Utc::now().to_rfc3339());
        manifest.total_files = manifest.files.len() as u64;
        manifest.total_bytes = manifest.files.values().map(|e| e.size_bytes).sum();
        manifest.save(&self.config.manifest_path())?;

        if total_copied > 0 {
            tracing::info!(
                "[SOVEREIGN_STACK] Scan complete: {} files copied, {} skipped (total in manifest: {})",
                total_copied,
                total_skipped,
                manifest.total_files
            );
        } else {
            tracing::debug!(
                "[SOVEREIGN_STACK] Scan complete: no new files ({} skipped, {} total)",
                total_skipped,
                manifest.total_files
            );
        }

        Ok(())
    }

    /// Walk a source directory and copy new/modified files to the destination.
    /// Returns (copied_count, skipped_count).
    async fn scrape_directory(
        &self,
        source: &Path,
        dest: &Path,
        volume_name: &str,
        manifest: &mut Manifest,
    ) -> Result<(u64, u64)> {
        let mut copied: u64 = 0;
        let mut skipped: u64 = 0;
        let mut stack: Vec<PathBuf> = vec![source.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("[SOVEREIGN_STACK] Cannot read {:?}: {}", dir, e);
                    continue;
                }
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };

                if path.is_dir() {
                    if should_skip_dir(&name) {
                        skipped += 1;
                        continue;
                    }
                    stack.push(path);
                } else if path.is_file() {
                    if should_skip(&name) {
                        skipped += 1;
                        continue;
                    }

                    // Get relative path from source root
                    let rel_path = match path.strip_prefix(source) {
                        Ok(r) => r.to_string_lossy().replace('\\', "/"),
                        Err(_) => continue,
                    };

                    // Stability check: skip files modified in last 5 seconds
                    let metadata = match tokio::fs::metadata(&path).await {
                        Ok(m) => m,
                        Err(_) => {
                            skipped += 1;
                            continue;
                        }
                    };

                    if let Ok(modified) = metadata.modified() {
                        if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                            if elapsed < Duration::from_secs(5) {
                                skipped += 1;
                                continue;
                            }
                        }
                    }

                    let source_size = metadata.len();
                    let dest_path = dest.join(&rel_path);

                    // Check if we need to copy (new file or size changed)
                    let manifest_key = format!("{}:{}", volume_name, rel_path);
                    let needs_copy = if let Some(existing) = manifest.files.get(&manifest_key) {
                        existing.size_bytes != source_size
                    } else {
                        // Also check if dest file exists with same size (from prior runs)
                        match tokio::fs::metadata(&dest_path).await {
                            Ok(dest_meta) => dest_meta.len() != source_size,
                            Err(_) => true,
                        }
                    };

                    if !needs_copy {
                        skipped += 1;
                        continue;
                    }

                    // Create parent directories at destination
                    if let Some(parent) = dest_path.parent() {
                        if let Err(e) = tokio::fs::create_dir_all(parent).await {
                            tracing::warn!(
                                "[SOVEREIGN_STACK] Failed to create dir {:?}: {}",
                                parent,
                                e
                            );
                            continue;
                        }
                    }

                    // Copy the file
                    match tokio::fs::copy(&path, &dest_path).await {
                        Ok(_) => {
                            // Compute SHA-256 hash (on blocking thread to avoid starving async runtime)
                            let path_for_hash = path.clone();
                            let hash =
                                tokio::task::spawn_blocking(move || compute_sha256(&path_for_hash))
                                    .await
                                    .unwrap_or_else(|_| Ok("hash_error".to_string()))
                                    .unwrap_or_else(|_| "hash_error".to_string());

                            manifest.files.insert(
                                manifest_key,
                                ManifestEntry {
                                    relative_path: rel_path,
                                    sha256: hash,
                                    size_bytes: source_size,
                                    source_volume: volume_name.to_string(),
                                    copied_at: chrono::Utc::now().to_rfc3339(),
                                },
                            );

                            copied += 1;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "[SOVEREIGN_STACK] Failed to copy {:?} -> {:?}: {}",
                                path,
                                dest_path,
                                e
                            );
                            skipped += 1;
                        }
                    }
                }
            }
        }

        Ok((copied, skipped))
    }
}

/// Compute SHA-256 hash of a file
fn compute_sha256(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open for hashing: {:?}", path))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = std::io::Read::read(&mut file, &mut buffer)
            .with_context(|| format!("Failed to read for hashing: {:?}", path))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
