// Simplified authentication service using bcrypt and sessions
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;

/// Simple auth service for internal use
pub struct AuthService;

impl AuthService {
    pub fn new() -> Self {
        Self
    }

    /// Hash a password using bcrypt
    pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
        hash(password, DEFAULT_COST)
    }

    /// Verify a password against a hash
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
        verify(password, hash)
    }

    /// Generate a new session ID
    pub fn generate_session_id() -> String {
        Uuid::new_v4().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash = AuthService::hash_password(password).unwrap();
        
        // Verify correct password
        assert!(AuthService::verify_password(password, &hash).unwrap());
        
        // Verify wrong password fails
        assert!(!AuthService::verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    #[ignore]
    fn test_generate_admin_hash() {
        let password = "admin123";
        let hash = AuthService::hash_password(password).unwrap();
        println!("Hash for admin123: {}", hash);
        assert!(AuthService::verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_session_id_generation() {
        let session1 = AuthService::generate_session_id();
        let session2 = AuthService::generate_session_id();
        
        // Each session ID should be unique
        assert_ne!(session1, session2);
        
        // Should be valid UUIDs
        assert!(Uuid::parse_str(&session1).is_ok());
        assert!(Uuid::parse_str(&session2).is_ok());
    }
}
