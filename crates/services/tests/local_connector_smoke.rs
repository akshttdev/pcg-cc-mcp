//! Smoke test for LocalFolderConnector against the real Sirak Studios
//! Dropbox tree on this master node. `#[ignore]`d so `cargo test` doesn't
//! run it automatically — invoke with:
//!
//!     cargo test -p services --test local_connector_smoke -- --ignored --nocapture
//!
//! Each test reports file/folder counts + total bytes to stdout so the
//! operator can eyeball-compare against the Dropbox client's stats.

use services::services::storage::{connectors::local::LocalFolderConnector, StorageConnector};

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
        println!(
            "  first file:  {} ({} bytes, mime={:?})",
            first.path, first.size_bytes, first.mime_type
        );
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
    walk_and_report(
        "Sirak Studios Dropbox — full personal Dropbox",
        SIRAK_DROPBOX_ROOT,
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn smoke_sirak_team() {
    walk_and_report("Sirak Studios Team — 641 GB / ~395k files", SIRAK_TEAM).await;
}

/// Volume-resolution contract: an absolute path stored in
/// `cloud_files.storage_volume` must round-trip through
/// `utils::volume::resolve_volume_path` to an existing file on disk. This is
/// what wires LocalFolderConnector-synced rows to the existing
/// `/org-cloud/{org_id}/files/{file_id}/download` route.
///
/// Not `#[ignore]`d — runs in CI as long as the Sirak data is on E:.
#[tokio::test]
async fn smoke_volume_resolution_for_local_paths() {
    use std::path::Path;

    use utils::volume::{resolve_volume_path, volume_base_path};

    if !Path::new(SIRAK_PERSONAL).exists() {
        eprintln!("skipping: {SIRAK_PERSONAL} not present on this machine");
        return;
    }

    // Base path passthrough: a literal absolute path should resolve to itself.
    let base = volume_base_path(SIRAK_PERSONAL).expect("absolute path must resolve");
    assert_eq!(
        base.to_string_lossy().replace('\\', "/"),
        SIRAK_PERSONAL,
        "volume_base_path of an absolute path should be the path itself"
    );

    // Walk one file out of the tree so we have a real file_path to resolve.
    let conn = LocalFolderConnector::new();
    let batch = conn
        .sync_changes("", None, Some(SIRAK_PERSONAL))
        .await
        .expect("walk should succeed");
    let first_file = batch
        .changes
        .iter()
        .find(|c| !c.is_folder)
        .expect("at least one file under SIRAK_PERSONAL");

    let resolved =
        resolve_volume_path(SIRAK_PERSONAL, &first_file.path).expect("path should resolve");
    assert!(
        resolved.exists(),
        "resolved download path {resolved:?} must exist on disk"
    );

    println!();
    println!("── Volume resolution smoke ─────────────────────────");
    println!("  storage_volume: {SIRAK_PERSONAL}");
    println!("  file_path:      {}", first_file.path);
    println!("  resolved:       {resolved:?}");
    println!("  exists on disk: {}", resolved.exists());

    // Path-traversal defence: a `..` in file_path must be rejected.
    let evil = resolve_volume_path(SIRAK_PERSONAL, "../etc/passwd");
    assert!(evil.is_err(), "../ in file_path must be rejected");
}
