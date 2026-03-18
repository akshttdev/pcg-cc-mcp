# Power Club Global - Decentralized Compute Client

Transform your Mac or Linux computer into a node in the Power Club Global decentralized compute network and earn VIBE tokens!

## 🚀 Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/KingBodhi/pcg-cc-mcp/main/install-pcg-client.sh | bash
```

## 💰 Two Separate Wallets

### Device Wallet (Automatic)
- **Purpose**: Receives VIBE rewards for compute contributions
- **Location**: `~/.apn/node_identity.json`
- **Created**: Automatically on first run
- **Unique**: Each device has its own wallet

### User Wallet (Dashboard)
- **Purpose**: Personal VIBE balance for using services
- **Managed**: Through the web dashboard
- **Login**: Required to access dashboard features
- **Separate**: From device rewards

**Example**: User "Bonomotion" has their own wallet in the dashboard, but the Bonomotion device (computer) has a separate wallet for earning compute rewards.

## 📦 What Gets Installed

- **PCG Dashboard** (`server`): Web interface + orchestrator
- **APN Node** (`apn_node`): Network peer for distributed compute
- **Device Identity**: Unique wallet for this device
- **Auto-Start**: Services start automatically

## 🎯 Quick Start

### Start the Client
```bash
pcg-client
```

### Access Dashboard
Open browser to: **http://localhost:58297**

### Enable Auto-Start

**Linux:**
```bash
systemctl --user enable pcg-client
systemctl --user start pcg-client
```

**Mac:**
```bash
launchctl load ~/Library/LaunchAgents/com.powerclubglobal.client.plist
```

## 💎 Earning VIBE

Your device earns VIBE by:
- **Uptime**: Staying connected to the network
- **Compute**: Processing tasks
- **GPU**: Bonus multiplier for GPU availability
- **Storage**: Providing distributed storage

Rewards are automatically sent to your device wallet!

## 📊 Check Your Device Wallet

```bash
cat ~/.apn/node_identity.json
```

## 🔧 Manual Start (Without Auto-Start)

```bash
~/.pcg-client/bin/pcg-client
```

## 📁 File Locations

- **Installation**: `~/.pcg-client/`
- **Binaries**: `~/.pcg-client/bin/`
- **Data**: `~/.pcg-client/data/`
- **Device Identity**: `~/.apn/node_identity.json`
- **Logs** (Mac): `~/.pcg-client/data/client.log`
- **Logs** (Linux): `journalctl --user -u pcg-client`

## 🔄 Update Client

```bash
cd ~/.pcg-client/repo
git pull origin main
cargo build --release --bin server --bin apn_node
cp target/release/{server,apn_node} ~/.pcg-client/bin/
```

Then restart the service.

## 🛑 Stop Client

**Linux:**
```bash
systemctl --user stop pcg-client
```

**Mac:**
```bash
launchctl unload ~/Library/LaunchAgents/com.powerclubglobal.client.plist
```

**Manual:**
```bash
pkill -f pcg-client
```

## 🗑️ Uninstall

```bash
# Stop services first (see above), then:
rm -rf ~/.pcg-client
rm ~/.config/systemd/user/pcg-client.service  # Linux
rm ~/Library/LaunchAgents/com.powerclubglobal.client.plist  # Mac

# Optional: Remove device identity (loses wallet!)
# rm -rf ~/.apn
```

⚠️ **Warning**: Deleting `~/.apn/` removes your device wallet and accumulated VIBE! Back up the mnemonic first!

## 🆘 Troubleshooting

### Check if Running
```bash
ps aux | grep pcg-client
```

### View Logs

**Linux:**
```bash
journalctl --user -u pcg-client -f
```

**Mac:**
```bash
tail -f ~/.pcg-client/data/client.log
```

### Check Network Connection
```bash
# Should see heartbeats every 30 seconds
tail -f ~/.pcg-client/data/client.log | grep heartbeat
```

### Reset Device Identity
```bash
# ⚠️ This creates a NEW wallet (loses old rewards!)
rm -rf ~/.apn
# Restart client to generate new identity
```

## 🌐 Network Status

Your device will appear on the network as:
- **Node ID**: `apn_xxxxxxxx` (from wallet address)
- **Hostname**: Your computer's hostname
- **Status**: Active when connected

## 🔐 Security

- Device keys stored in `~/.apn/` with `0o700` permissions
- Mnemonic phrase automatically backed up
- **BACKUP YOUR MNEMONIC**: Found in `~/.apn/node_identity.json`

## 📞 Support

- Website: https://powerclubglobal.com
- Issues: https://github.com/KingBodhi/pcg-cc-mcp/issues
- Discord: [Coming Soon]

## 📄 License

MIT License - See LICENSE file

---

**Power Club Global** - Decentralized Compute, Distributed Rewards
