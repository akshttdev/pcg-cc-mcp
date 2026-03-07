//! Static constants for well-known database entities.
//!
//! These UUIDs are seeded by migrations and should never change.

use uuid::Uuid;

/// The Powerclub Global project — feedback/bug reports land here.
///
/// UUID: 05abaaf5-b249-4d1c-a980-c27aa095f579
pub const BUGREPORTS_PROJECT_ID: Uuid = Uuid::from_u128(0x05abaaf5_b249_4d1c_a980_c27aa095f579);

/// The Bug Board within the Powerclub Global project.
///
/// UUID: f91564ab-ae26-0bfd-f3f8-ad627cf442c0
pub const BUGREPORTS_BOARD_ID: Uuid = Uuid::from_u128(0xf91564ab_ae26_0bfd_f3f8_ad627cf442c0);

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
            "05abaaf5-b249-4d1c-a980-c27aa095f579"
        );
    }

    #[test]
    fn test_bugreports_board_id() {
        assert_eq!(
            BUGREPORTS_BOARD_ID.to_string(),
            "f91564ab-ae26-0bfd-f3f8-ad627cf442c0"
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
