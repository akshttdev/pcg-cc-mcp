#!/usr/bin/env python3
"""
Add Sirak and Bonomotion users to PCG Dashboard
"""

import sqlite3
import bcrypt
import uuid
from datetime import datetime

# Database path - THIS IS THE ACTUAL DATABASE THE BACKEND USES!
DB_PATH = "/home/pythia/.local/share/duck-kanban/db.sqlite"

# Users to add
users = [
    {
        "username": "Sirak",
        "password": "Sirak123",
        "email": "sirak@powerclubglobal.com",
        "full_name": "Sirak",
        "is_admin": True,  # Give admin access
        "is_active": True
    },
    {
        "username": "Bonomotion",
        "password": "bonomotion123",
        "email": "bonomotion@powerclubglobal.com",
        "full_name": "Bonomotion",
        "is_admin": True,  # Give admin access
        "is_active": True
    }
]

def hash_password(password: str) -> str:
    """Generate bcrypt hash for password"""
    # Generate salt and hash the password
    salt = bcrypt.gensalt(rounds=12)
    hashed = bcrypt.hashpw(password.encode('utf-8'), salt)
    return hashed.decode('utf-8')

def add_users():
    """Add users to database"""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    print("=" * 60)
    print("Adding users to PCG Dashboard")
    print("=" * 60)

    for user in users:
        # Check if user already exists
        cursor.execute("SELECT username FROM users WHERE username = ?", (user["username"],))
        existing = cursor.fetchone()

        if existing:
            print(f"\n⚠️  User '{user['username']}' already exists - updating password...")
            # Update password
            password_hash = hash_password(user["password"])
            cursor.execute(
                """UPDATE users
                   SET password_hash = ?, email = ?, full_name = ?, is_admin = ?, is_active = ?
                   WHERE username = ?""",
                (password_hash, user["email"], user["full_name"],
                 user["is_admin"], user["is_active"], user["username"])
            )
            print(f"✅ Updated user '{user['username']}'")
        else:
            # Create new user
            user_id = str(uuid.uuid4())
            password_hash = hash_password(user["password"])
            created_at = datetime.utcnow().isoformat()

            cursor.execute(
                """INSERT INTO users
                   (id, username, email, full_name, password_hash, is_admin, is_active, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (user_id, user["username"], user["email"], user["full_name"],
                 password_hash, user["is_admin"], user["is_active"], created_at, created_at)
            )
            print(f"\n✅ Created user '{user['username']}'")

        print(f"   Username: {user['username']}")
        print(f"   Password: {user['password']}")
        print(f"   Email: {user['email']}")
        print(f"   Admin: {'Yes' if user['is_admin'] else 'No'}")

    conn.commit()

    # Show all users
    print("\n" + "=" * 60)
    print("All users in database:")
    print("=" * 60)
    cursor.execute("SELECT username, email, is_admin, is_active FROM users")
    for row in cursor.fetchall():
        admin_badge = " [ADMIN]" if row[2] else ""
        active_badge = " [ACTIVE]" if row[3] else " [INACTIVE]"
        print(f"  - {row[0]:<20} {row[1]:<30}{admin_badge}{active_badge}")

    conn.close()

    print("\n" + "=" * 60)
    print("✅ Users added successfully!")
    print("=" * 60)
    print("\n📝 Login Credentials:")
    print("   Username: Sirak         | Password: Sirak123")
    print("   Username: Bonomotion    | Password: bonomotion123")
    print("\n🌐 Access dashboard at: https://dashboard.powerclubglobal.com")
    print("=" * 60)

if __name__ == "__main__":
    add_users()
