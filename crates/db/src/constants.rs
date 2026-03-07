//! Static constants for well-known database entities.
//!
//! These UUIDs are seeded by migrations and should never change.

use uuid::Uuid;

/// The Bug Reports project is used to track issues with the dashboard itself.
/// Users can submit bug reports which become tasks in this project.
///
/// UUID: 00000000-0000-0000-0000-000000000001
pub const BUGREPORTS_PROJECT_ID: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000001);

/// The default board within the Bug Reports project.
///
/// UUID: 00000000-0000-0000-0000-000000000002
pub const BUGREPORTS_BOARD_ID: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000002);

/// The default admin user for local development.
/// Only seeded if no users exist in the database.
///
/// UUID: 00000000-0000-0000-0000-000000000099
/// Username: admin
/// Password: admin123
pub const DEV_ADMIN_USER_ID: Uuid = Uuid::from_u128(0x00000000_0000_0000_0000_000000000099);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bugreports_project_id() {
        assert_eq!(
            BUGREPORTS_PROJECT_ID.to_string(),
            "00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn test_bugreports_board_id() {
        assert_eq!(
            BUGREPORTS_BOARD_ID.to_string(),
            "00000000-0000-0000-0000-000000000002"
        );
    }

    #[test]
    fn test_dev_admin_user_id() {
        assert_eq!(
            DEV_ADMIN_USER_ID.to_string(),
            "00000000-0000-0000-0000-000000000099"
        );
    }
}
