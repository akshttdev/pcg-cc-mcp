#!/bin/bash
# Fix Dashboard Configuration - Run with sudo

if [ "$EUID" -ne 0 ]; then
    echo "Please run with sudo: sudo ./fix-dashboard-config.sh"
    exit 1
fi

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Fixing PCG Dashboard Configuration                     ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# 1. Update nginx config to use port 58297
echo "→ Updating nginx configuration..."
sed -i 's/proxy_pass http:\/\/127.0.0.1:3001;/proxy_pass http:\/\/127.0.0.1:58297;/g' /etc/nginx/sites-available/pcg-dashboard

# 2. Test nginx config
echo "→ Testing nginx configuration..."
nginx -t
if [ $? -ne 0 ]; then
    echo "❌ Nginx config test failed!"
    exit 1
fi

# 3. Reload nginx
echo "→ Reloading nginx..."
systemctl reload nginx

# 4. Check what's on port 3001
echo ""
echo "→ Checking port 3001..."
lsof -i :3001 | head -5

# 5. Install Brave browser
echo ""
echo "→ Installing Brave browser..."
curl -fsSLo /usr/share/keyrings/brave-browser-archive-keyring.gpg https://brave-browser-apt-release.s3.brave.com/brave-browser-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/brave-browser-archive-keyring.gpg] https://brave-browser-apt-release.s3.brave.com/ stable main" | tee /etc/apt/sources.list.d/brave-browser-release.list
apt update
apt install -y brave-browser

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ✅ Configuration Fixed!                                ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Changes made:                                          ║"
echo "║  • Nginx now proxies /api/ to port 58297                ║"
echo "║  • Brave browser installed                              ║"
echo "║                                                          ║"
echo "║  Next steps:                                            ║"
echo "║  1. Refresh dashboard in browser                        ║"
echo "║  2. Projects should now load correctly                  ║"
echo "╚══════════════════════════════════════════════════════════╝"
