#!/usr/bin/env python3
"""
Test login credentials for Sirak and Bonomotion
"""

import sqlite3
import bcrypt

DB_PATH = "/home/pythia/pcg-cc-mcp/dev_assets/db.sqlite"

def test_password(username, password):
    """Test if password matches hash in database"""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("SELECT password_hash, is_admin, is_active FROM users WHERE username = ?", (username,))
    result = cursor.fetchone()

    if not result:
        print(f"❌ User '{username}' not found in database")
        return False

    password_hash, is_admin, is_active = result

    # Test password
    try:
        matches = bcrypt.checkpw(password.encode('utf-8'), password_hash.encode('utf-8'))

        print(f"\n{'='*60}")
        print(f"Testing: {username}")
        print(f"{'='*60}")
        print(f"  Username: {username}")
        print(f"  Password: {password}")
        print(f"  Hash: {password_hash[:50]}...")
        print(f"  Password Match: {'✅ YES' if matches else '❌ NO'}")
        print(f"  Is Admin: {'✅ YES' if is_admin else '❌ NO (regular user)'}")
        print(f"  Is Active: {'✅ YES' if is_active else '❌ NO'}")

        return matches
    except Exception as e:
        print(f"❌ Error testing password: {e}")
        return False
    finally:
        conn.close()

if __name__ == "__main__":
    print("\n" + "="*60)
    print("PCG Dashboard - Login Test")
    print("="*60)

    test_password("Sirak", "Sirak123")
    test_password("Bonomotion", "bonomotion123")

    print("\n" + "="*60)
    print("All Users in Database:")
    print("="*60)

    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("SELECT username, email, is_admin, is_active FROM users")
    for row in cursor.fetchall():
        admin = "ADMIN" if row[2] else "USER"
        active = "ACTIVE" if row[3] else "INACTIVE"
        print(f"  {row[0]:<20} {row[1]:<35} [{admin}] [{active}]")
    conn.close()

    print("="*60)
