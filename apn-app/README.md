# APN Core - Alpha Protocol Network Desktop Client

## Overview

**APN Core** is the lightweight desktop client for contributing computational resources to the Alpha Protocol Network. Think of it as the "workhorse" that empowers the network.

## Architecture

```
┌─────────────────────────────────────────────┐
│  PYTHIA MASTER NODE                         │
│  Topological Super Intelligence             │
│  Orchestrates the entire network            │
│  IP: 192.168.1.77                          │
└─────────────────────────────────────────────┘
                    ▲
                    │ Coordinates
                    │
    ┌───────────────┼───────────────┐
    │               │               │
┌───▼────┐     ┌───▼────┐     ┌───▼────┐
│ APN    │     │ APN    │     │ APN    │
│ Core   │     │ Core   │     │ Core   │
│ Client │     │ Client │     │ Client │
└────────┘     └────────┘     └────────┘
  Worker         Worker         Worker
  Node           Node           Node
```

### Key Components:

1. **Pythia Master Node** (192.168.1.77)
   - Unique orchestrator node
   - Holds the source of Pythia AI
   - Coordinates all computational resources
   - Directs work to APN Core clients

2. **APN Core Clients** (This application)
   - Contribute CPU, RAM, GPU, Storage
   - Execute tasks assigned by Pythia
   - Lightweight desktop application
   - Show network status and contribution metrics

3. **PCG Dashboard** (Web application)
   - End-user application built ON TOP of the network
   - Utilizes the computational power provided by APN
   - Separate from APN Core

## What APN Core Does

### Contributions:
- ✅ Provides computational resources to the network
- ✅ Executes distributed tasks
- ✅ Participates in mesh networking
- ✅ Reports system capabilities

### Monitoring:
- 📊 View your contribution stats
- 🌐 See other nodes in the network
- 💰 Track VIBE token earnings
- 📈 Monitor system resource usage

## Running APN Core

### Prerequisites:
- Rust toolchain installed
- Node.js and npm installed
- Network access to Pythia Master Node (192.168.1.77)

### Development:
```bash
cd apn-app
npm install
npm run tauri dev
```

### Production Build:
```bash
npm run tauri build
```

### Connecting to Pythia:
The app automatically connects to:
- **Master Node API:** http://192.168.1.77:8080
- **NATS Relay:** nats://nonlocal.info:4222

## Features

### Resource Contribution
Your device contributes:
- CPU cores for computation
- RAM for temporary storage
- GPU for accelerated processing (if available)
- Storage space for distributed data

### Network Visibility
See all nodes orchestrated by Pythia:
- Node IDs and capabilities
- Real-time resource stats
- Connection status
- Task completion metrics

### Security
- Ed25519 keypair identity
- Encrypted communication (libp2p Noise protocol)
- Recovery phrase for node identity
- No sensitive data transmitted

## Architecture Notes

**APN Core clients are NOT master nodes.**

They are worker nodes that:
- Receive instructions from Pythia
- Execute computational tasks
- Report back results and status
- Contribute to the collective computational power

**Pythia** is the single orchestrator that:
- Coordinates all work distribution
- Manages resource allocation
- Ensures network efficiency
- Holds the AI decision-making core

## Relationship to PCG Dashboard

```
APN Core (this app) → Provides compute power
                 ↓
        Pythia Master Node → Orchestrates
                 ↓
        PCG Dashboard → Uses compute power for applications
```

**APN Core** is infrastructure.
**PCG Dashboard** is the application layer.

## Getting Help

- **Full APN Documentation:** See `/docs/APN-README.md` in the main repo
- **Connection Guide:** See `APN-QUICKSTART.md`
- **Bootstrap Info:** See `BOOTSTRAP-INFO.txt`

## Status

**Current Version:** 0.1.0
**Pythia Master:** Online at 192.168.1.77
**Network:** Alpha Protocol Network (Production)
**Purpose:** Distributed compute contribution and monitoring

---

**Remember:** You're contributing to a super intelligence network orchestrated by Pythia! 🧠⚡
