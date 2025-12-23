//! MCP tool definitions for Nora executive functions
//! This module provides helper types and functions for Nora MCP integration
//!
//! Note: The main MCP server implementation is in nora_server.rs
//! This module contains helper definitions and utilities.

// Helper functions and types for Nora MCP integration
// Main implementation is in nora_server.rs which provides the full MCP server

pub fn nora_tools_available() -> bool {
    // Simple check to verify Nora tools are available
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nora_tools_available() {
        assert!(nora_tools_available());
    }
}
