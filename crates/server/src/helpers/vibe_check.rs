use db::models::system_settings::SystemSetting;
use sqlx::SqlitePool;

/// Returns true if VIBE balance checks should be bypassed.
/// Only active in debug builds AND when the admin has enabled the bypass setting.
pub async fn is_vibe_bypass_active(pool: &SqlitePool) -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }
    SystemSetting::is_vibe_bypass_enabled(pool).await
}
