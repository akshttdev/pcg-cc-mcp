//! Smoke test for LocalFolderConnector against the real Sirak Studios
//! Dropbox tree on this master node. `#[ignore]`d so `cargo test` doesn't
//! run it automatically — invoke with:
//!
//!     cargo test -p services --test local_connector_smoke -- --ignored --nocapture
//!
//! Each test reports file/folder counts + total bytes to stdout so the
//! operator can eyeball-compare against the Dropbox client's stats.

use services::services::storage::{StorageConnector, connectors::local::LocalFolderConnector};

const SIRAK_PERSONAL: &str = "E:/topos/Sirak Studios Dropbox/Sirak Studios (sirak)";
const SIRAK_DROPBOX_ROOT: &str = "E:/topos/Sirak Studios Dropbox";
const SIRAK_TEAM: &str = "E:/topos/Sirak Studios Team";

async fn walk_and_report(label: &str, root: &str) {
    if !std::path::Path::new(root).exists() {
        panic!("smoke test root not present on this machine: {root}");
    }

    let start = std::time::Instant::now();
    let conn = LocalFolderConnector::new();
    let batch = conn
        .sync_changes("", None, Some(root))
        .await
        .expect("sync_changes should succeed");
    let elapsed = start.elapsed();

    let files: Vec<_> = batch.changes.iter().filter(|c| !c.is_folder).collect();
    let folders: Vec<_> = batch.changes.iter().filter(|c| c.is_folder).collect();
    let total_bytes: i64 = files.iter().map(|c| c.size_bytes).sum();

    let mb = total_bytes as f64 / (1024.0 * 1024.0);
    let gb = mb / 1024.0;

    println!();
    println!("── {label} ─────────────────────────────────────────");
    println!("  root:        {root}");
    println!("  elapsed:     {:.2?}", elapsed);
    println!("  folders:     {}", folders.len());
    println!("  files:       {}", files.len());
    println!("  total bytes: {total_bytes} ({mb:.1} MB / {gb:.2} GB)");
    if let Some(first) = files.first() {
        println!("  first file:  {} ({} bytes, mime={:?})", first.path, first.size_bytes, first.mime_type);
    }
    if let Some(last) = files.last() {
        println!("  last file:   {} ({} bytes)", last.path, last.size_bytes);
    }
    println!("  next_cursor present: {}", batch.next_cursor.is_some());

    assert!(!files.is_empty(), "expected at least one file under {root}");
}

#[tokio::test]
#[ignore]
async fn smoke_sirak_personal_subdir() {
    walk_and_report("Sirak Studios (sirak) — personal subdir", SIRAK_PERSONAL).await;
}

#[tokio::test]
#[ignore]
async fn smoke_sirak_dropbox_root() {
    walk_and_report("Sirak Studios Dropbox — full personal Dropbox", SIRAK_DROPBOX_ROOT).await;
}

#[tokio::test]
#[ignore]
async fn smoke_sirak_team() {
    walk_and_report("Sirak Studios Team — 641 GB / ~395k files", SIRAK_TEAM).await;
}
