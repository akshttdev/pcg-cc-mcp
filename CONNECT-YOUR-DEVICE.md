# 🚀 Connect Your Device to Alpha Protocol Network

## The Fastest Way to Join

```bash
git clone https://github.com/KingBodhi/pcg-cc-mcp.git
cd pcg-cc-mcp
git checkout new
chmod +x setup-peer-node.sh
./setup-peer-node.sh
```

**When prompted, paste this bootstrap address:**

```
/ip4/192.168.1.77/tcp/4001/p2p/12D3KooWPoBpKG7vzM2ufLtHDVqenRKUvUu8DizsADhqqW7Z9du5
```

**Done!** Your device is now connected. 🎉

---

## What Happens Next

After running the setup:
1. ✅ Binary builds automatically (~2-5 minutes)
2. ✅ Node connects to Pythia Master
3. ✅ Resources detected (CPU, RAM, GPU, Storage)
4. ✅ Heartbeat starts (status every 30s)
5. ✅ Ready to receive distributed tasks

---

## Verify Connection

```bash
# Watch the logs
tail -f /tmp/apn_peer.log

# Look for these messages:
✅ "🟢 Node started"
✅ "🌐 Relay connected"
✅ "Collected resources"
```

---

## Alternative: Use Bootstrap Info File

Don't want to remember the address? Just copy from the file:

```bash
cat BOOTSTRAP-INFO.txt
```

Then paste when the setup script asks.

---

## Documentation

Choose your path:

| If you want to... | Read this |
|-------------------|-----------|
| **Connect in 3 steps** | [APN-QUICKSTART.md](APN-QUICKSTART.md) |
| **Understand the system** | [APN-README.md](APN-README.md) |
| **Troubleshoot issues** | [DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md) |
| **Copy bootstrap info** | [BOOTSTRAP-INFO.txt](BOOTSTRAP-INFO.txt) |
| **Manual setup** | [DEPLOYMENT-GUIDE.md#manual-setup](DEPLOYMENT-GUIDE.md) |

---

## Network Information

**Current Master Node:**
- Location: 192.168.1.77 (local network)
- NATS Relay: nats://nonlocal.info:4222
- Status: ✅ Online with resource reporting

**Your Node Will Report:**
- CPU cores and availability
- RAM total and available
- Storage space
- GPU model (if present)

All reported every 30 seconds. Your files remain private.

---

## Troubleshooting

**"Bootstrap address required"**
→ Copy the full address starting with `/ip4/...`

**"Connection refused"**
→ Verify master is online: `ping 192.168.1.77`

**"Build failed"**
→ Update Rust: `rustup update && cargo clean`

**"No GPU detected"**
→ This is normal! CPU, RAM, storage will still report

**More help:** See [DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md#troubleshooting)

---

## What's Shared

**Shared with network:**
- Number of CPU cores
- Total RAM
- Available storage space
- GPU model name (if detected)

**NOT shared:**
- Your files or data
- Process information
- Network traffic details
- Personal information

Everything is encrypted end-to-end with libp2p Noise protocol.

---

## After Connecting

```bash
# Check network status
./check-network-capacity.sh

# View all peers
grep "📨" /tmp/apn_peer.log

# Monitor resources
grep "Collected resources" /tmp/apn_peer.log

# Stop your node
kill $(cat /tmp/apn_peer.pid)
```

---

## Support

- **Logs:** Check `/tmp/apn_peer.log` for errors
- **Docs:** See [DEPLOYMENT-GUIDE.md](DEPLOYMENT-GUIDE.md)
- **Status:** Run `./check-network-capacity.sh`

---

**Ready to connect?** Just run the 4 commands at the top! 🚀
