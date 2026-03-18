#!/bin/bash
# Generate bcrypt hash for admin123 password
cd crates/db
cargo test test_generate_admin_hash -- --nocapture --ignored 2>&1 | grep "Hash for admin123:"
