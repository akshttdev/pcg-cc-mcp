#!/bin/bash
# Reset admin password for PCG Dashboard

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Reset Admin Password                                   ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Ask for new password
read -sp "Enter new admin password: " NEW_PASSWORD
echo ""
read -sp "Confirm password: " CONFIRM_PASSWORD
echo ""

if [ "$NEW_PASSWORD" != "$CONFIRM_PASSWORD" ]; then
    echo "❌ Passwords don't match!"
    exit 1
fi

if [ -z "$NEW_PASSWORD" ]; then
    echo "❌ Password cannot be empty!"
    exit 1
fi

echo ""
echo "→ Generating password hash..."

# Generate bcrypt hash using Python
HASH=$(python3 -c "import bcrypt; print(bcrypt.hashpw('$NEW_PASSWORD'.encode(), bcrypt.gensalt()).decode())")

if [ -z "$HASH" ]; then
    echo "❌ Failed to generate password hash. Installing bcrypt..."
    pip3 install bcrypt --quiet
    HASH=$(python3 -c "import bcrypt; print(bcrypt.hashpw('$NEW_PASSWORD'.encode(), bcrypt.gensalt()).decode())")
fi

echo "→ Updating database..."
sqlite3 /home/pythia/pcg-cc-mcp/dev_assets/db.sqlite "UPDATE users SET password_hash = '$HASH' WHERE username = 'admin';"

echo ""
echo "✅ Admin password reset successfully!"
echo ""
echo "You can now login with:"
echo "  Username: admin"
echo "  Password: [your new password]"
echo ""
echo "→ Restart the backend server to apply changes:"
echo "   kill \$(cat /tmp/pcg_backend.pid) && cd /home/pythia/pcg-cc-mcp && BACKEND_PORT=58297 ./target/release/server > /tmp/pcg_backend.log 2>&1 & echo \$! > /tmp/pcg_backend.pid"
