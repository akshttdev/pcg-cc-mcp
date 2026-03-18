# Alpha Protocol Network - Documentation Index

## Start Here

**New to APN?** → [getting-started/CONNECT-YOUR-DEVICE.md](getting-started/CONNECT-YOUR-DEVICE.md)

**Need bootstrap address?** → [BOOTSTRAP-INFO.txt](BOOTSTRAP-INFO.txt)

---

## Documentation by Purpose

### Getting Started
- **[getting-started/CONNECT-YOUR-DEVICE.md](getting-started/CONNECT-YOUR-DEVICE.md)** - Simplest way to connect (recommended)
- **[apn/APN-QUICKSTART.md](apn/APN-QUICKSTART.md)** - 3-step connection guide
- **[BOOTSTRAP-INFO.txt](BOOTSTRAP-INFO.txt)** - Copy-paste connection info

### Understanding APN
- **[apn/README-APN.md](apn/README-APN.md)** - Main APN entry point
- **[apn/APN-README.md](apn/APN-README.md)** - Complete features and architecture
- **[PYTHIA-MASTER-INFO-ENHANCED.txt](PYTHIA-MASTER-INFO-ENHANCED.txt)** - Master node details

### Deployment & Setup
- **[deployment/DEPLOYMENT-GUIDE.md](deployment/DEPLOYMENT-GUIDE.md)** - Full deployment documentation
- **[deployment/docker/](deployment/docker/)** - Docker deployment guides

### Architecture
- **[architecture/ORCHA_ARCHITECTURE_DIAGRAMS.md](architecture/ORCHA_ARCHITECTURE_DIAGRAMS.md)** - Architecture diagrams
- **[architecture/AGENTS.md](architecture/AGENTS.md)** - Agent services

### Core Features
- **[core-features/TOPSI_INTEGRATION_LOG.md](core-features/TOPSI_INTEGRATION_LOG.md)** - Topsi integration
- **[core-features/VIRTUAL_ENVIRONMENT_ROADMAP.md](core-features/VIRTUAL_ENVIRONMENT_ROADMAP.md)** - Virtual environment
- **[core-features/VOICE_ANALYTICS_QUICKSTART.md](core-features/VOICE_ANALYTICS_QUICKSTART.md)** - Voice analytics

### Sovereign Stack
- **[sovereign-stack/SOVEREIGN_ARCHITECTURE.md](sovereign-stack/SOVEREIGN_ARCHITECTURE.md)** - Sovereign architecture
- **[sovereign-stack/STORAGE_PROVIDER_ARCHITECTURE.md](sovereign-stack/STORAGE_PROVIDER_ARCHITECTURE.md)** - Storage provider

---

## Quick Reference

### Connect Command
```bash
git clone https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp
./scripts/setup-peer-node.sh
```

### Check Status
```bash
./scripts/check-network-capacity.sh
tail -f /tmp/apn_peer.log
```

---

**Last Updated:** 2026-03-18
**Status:** All documentation production ready — reorganized from root to docs/
