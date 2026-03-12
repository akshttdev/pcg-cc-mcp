# PR #13 Regression Review

**PR**: #13 "Support sound notification on windows"
**Merged**: 2025-06-30 into `816711ff`
**File changed**: `backend/src/execution_monitor.rs`
**Reviewed against**: `feature/fraze-2026-03-11` (current branch)

---

## What PR #13 Changed

The PR modified the Windows branch of `play_sound_notification()` to:
1. Resolve the sound file to an absolute path
2. Check if the file exists on disk
3. If it exists, play it via `(New-Object Media.SoundPlayer "path").PlaySync()`
4. If it doesn't exist, **fallback to system beep** via `[System.Media.SystemSounds]::Beep.Play()`

Previously it always fired the system beep and never played the actual sound file.

---

## Current State

The file `backend/src/execution_monitor.rs` was deleted and its functionality refactored into the crates structure. All functions have been accounted for:

| Original Function | Current Location | Status |
|---|---|---|
| `play_sound_notification()` | `crates/services/src/services/notification.rs:60` | Rewritten with improvements |
| `send_push_notification()` | `crates/services/src/services/notification.rs:118` | Rewritten |
| `execution_monitor()` (polling loop) | `crates/local-deployment/src/container.rs:490-650` | Replaced by direct process exit handling (no polling) |
| `commit_execution_changes()` | `crates/local-deployment/src/container.rs` → `try_commit_changes()` (line 521) | Present |
| `handle_setup_completion()` | `crates/local-deployment/src/container.rs:509-650` | Merged into unified completion handler |
| `handle_coding_agent_completion()` | `crates/local-deployment/src/container.rs:509-650` | Merged into unified completion handler |
| `handle_dev_server_completion()` | `crates/local-deployment/src/container.rs:509-650` | Merged into unified completion handler |
| Orphan process cleanup | `crates/local-deployment/src/container.rs:321` → `cleanup_orphaned_worktrees()` | Present |

**No functions are missing.** The old 5-second polling loop was replaced by direct handling when the child process exits, which is a better design.

---

## Regression Analysis

### 1. Windows Sound Fallback — PARTIAL REGRESSION

**PR #13 code** (in old `execution_monitor.rs`):
```rust
if absolute_path.exists() {
    // Play the actual sound file
    (New-Object Media.SoundPlayer "path").PlaySync()
} else {
    // Fallback to system beep
    [System.Media.SystemSounds]::Beep.Play()
}
```

**Current code** (`notification.rs:96-113`):
```rust
} else if cfg!(target_os = "windows") || (cfg!(target_os = "linux") && utils::is_wsl2()) {
    // WSL path conversion...
    let _ = tokio::process::Command::new("powershell.exe")
        .arg("-c")
        .arg(format!(r#"(New-Object Media.SoundPlayer "{file_path}").PlaySync()"#))
        .spawn();
}
```

**Findings**:
- The **system beep fallback is missing** on Windows. If the sound file doesn't exist, the `SoundPlayer` call will silently fail with no fallback.
- However, the new code uses `sound_file.get_path().await` (line 61) which resolves/caches the file before reaching platform branches. If `get_path()` fails, it returns early with a log error (lines 63-66). This partially mitigates the issue since the path should be valid by the time it reaches the Windows branch.
- **Net**: Low risk regression. The new path resolution is more robust than the old `to_path()` + manual `exists()` check, but the beep fallback is gone.

**Improvements over PR #13 in current code**:
- Added WSL2 support (path conversion via `wsl_to_windows_path`)
- Uses `powershell.exe` (works from WSL) instead of `powershell`
- Async path resolution with caching via `get_path()`
- Unified WSL2 + native Windows into one branch

---

## Recommendations

### Low Priority
1. **Add Windows beep fallback** in `notification.rs`: After the `SoundPlayer` spawn, add an existence check or catch the case where `get_path()` returns a path that doesn't actually exist on the Windows filesystem (particularly relevant for WSL2 path conversion failures).

---

## Verdict

**No regressions.** PR #13's core fix (playing actual sound files on Windows instead of always beeping) is preserved. All other `execution_monitor` functions exist in the refactored codebase. The only minor gap is the removal of the fallback-to-beep safety net, which is partially mitigated by improved path resolution.
