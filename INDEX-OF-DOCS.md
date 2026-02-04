# 📚 Alpha Protocol Network - Documentation Index

## 🎯 Start Here

**New to APN?** → [CONNECT-YOUR-DEVICE.md](CONNECT-YOUR-DEVICE.md)

**Need bootstrap address?** → [BOOTSTRAP-INFO.txt](BOOTSTRAP-INFO.txt)

---

## 📖 Documentation by Purpose

### Getting Started
- **[CONNECT-YOUR-DEVICE.md](CONNECT-YOUR-DEVICE.md)** - Simplest way to connect (recommended)
- **[APN-QUICKSTART.md](APN-QUICKSTART.md)** - 3-step connection guide
- **[BOOTSTRAP-INFO.txt](BOOTSTRAP-INFO.txt)** - Copy-paste connection info

### Understanding APN
- **[README-APN.md](README-APN.md)** - Main APN entry point
- **[APN-README.md](APN-README.md)** - Complete features and architecture
- **[PYTHIA-MASTER-INFO-ENHANCED.txt](PYTHIA-MASTER-INFO-ENHANCED.txt)** - Master node details

### Deployment & Setup
- **[DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md)** - Full deployment documentation
- **[setup-peer-node.sh](setup-peer-node.sh)** - Automated peer setup script
- **[setup-pythia-master-enhanced.sh](setup-pythia-master-enhanced.sh)** - Master node setup script

### Monitoring & Operations
- **[check-network-capacity.sh](check-network-capacity.sh)** - Network capacity monitor

### GitHub Quick Access
- **[.github/README-CONNECT.md](.github/README-CONNECT.md)** - GitHub-specific quick connect

---

## 🚀 Quick Reference

### Connect Command
```bash
git clone https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp && git checkout new
./setup-peer-node.sh
```

### Bootstrap Address
```
/ip4/192.168.1.77/tcp/4001/p2p/12D3KooWGaopz8uKs5ikXxD4yy5wQDn5yue2Q38T81pMtLbxMVvt
```

### Check Status
```bash
./check-network-capacity.sh
tail -f /tmp/apn_peer.log
```

---

## 📋 Documentation Summary

| File | Size | Purpose |
|------|------|---------|
| CONNECT-YOUR-DEVICE.md | 4.5 KB | Fastest connection guide |
| APN-QUICKSTART.md | 3.2 KB | 3-step quick start |
| APN-README.md | 9.9 KB | Complete APN docs |
| BOOTSTRAP-INFO.txt | 4.6 KB | Bootstrap info only |
| DEPLOYMENT-GUIDE.md | 8.1 KB | Full deployment guide |
| README-APN.md | 2.3 KB | APN overview |
| PYTHIA-MASTER-INFO-ENHANCED.txt | 3.2 KB | Master node details |

---

## 🔍 Find What You Need

**I want to connect my device**
→ [CONNECT-YOUR-DEVICE.md](CONNECT-YOUR-DEVICE.md)

**I need the bootstrap address**
→ [BOOTSTRAP-INFO.txt](BOOTSTRAP-INFO.txt)

**I want to understand how it works**
→ [APN-README.md](APN-README.md)

**I'm having connection issues**
→ [DEPLOYMENT-GUIDE.md#troubleshooting](DEPLOYMENT-GUIDE.md)

**I want to see network capacity**
→ Run `./check-network-capacity.sh`

**I need master node information**
→ [PYTHIA-MASTER-INFO-ENHANCED.txt](PYTHIA-MASTER-INFO-ENHANCED.txt)

**I want manual setup instructions**
→ [DEPLOYMENT-GUIDE.md#manual-setup](DEPLOYMENT-GUIDE.md)

---

## 🎯 Recommended Reading Order

1. **[CONNECT-YOUR-DEVICE.md](CONNECT-YOUR-DEVICE.md)** - Get connected first
2. **[APN-README.md](APN-README.md)** - Understand what you're running
3. **[DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md)** - Deep dive if needed

---

**Last Updated:** 2026-02-04
**Status:** All documentation production ready ✅
