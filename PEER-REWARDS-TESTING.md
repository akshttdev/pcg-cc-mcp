# 🧪 Peer Rewards System - Testing Guide

## Overview

This guide walks through testing the complete peer reward distribution system from heartbeat tracking to blockchain distribution.

---

## Prerequisites

### 1. Fund the Rewards Wallet

Transfer **110,000,000 VIBE** to the rewards wallet:

```
0xa3d8281b3a62cd5ca1e1ec68c1a60aea2844d60591f7b4b1fb0ebb878e321c9a
```

Verify on Aptos Explorer:
https://explorer.aptoslabs.com/account/0xa3d8281b3a62cd5ca1e1ec68c1a60aea2844d60591f7b4b1fb0ebb878e321c9a?network=testnet

### 2. Environment Setup

Ensure `.env` has rewards wallet configured:
```bash
REWARDS_WALLET_ADDRESS=0xa3d8281b3a62cd5ca1e1ec68c1a60aea2844d60591f7b4b1fb0ebb878e321c9a
REWARDS_WALLET_SEED="garbage figure spatial guard mesh write expect nature future gym scene glimpse"
```

### 3. Database Migration

Migration already applied during setup. Verify tables exist:
```bash
sqlite3 dev_assets/db.sqlite "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'peer%';"
```

Should show:
- peer_nodes
- peer_contributions
- peer_rewards
- reward_batches
- peer_wallet_balances
- reward_distribution_log

---

## Phase 1: Heartbeat Tracking Test

### Start the Master Node (Pythia)

```bash
cd /home/pythia/pcg-cc-mcp
./start-pythia-master.sh  # Or start APN node manually
```

### Verify Heartbeat Tracking

Check that the reward tracker is running:
```bash
tail -f /tmp/apn_node.log | grep "Heartbeat\|Reward"
```

Expected output:
```
💓 Heartbeat from apn_814d37f4
💰 Peer apn_814d37f4 earned 0.2 VIBE (2x multiplier)
💎 Created reward for apn_814d37f4: 0.1 VIBE
```

### Check Database

Verify peer node was registered:
```bash
sqlite3 dev_assets/db.sqlite "SELECT node_id, wallet_address, last_heartbeat_at, gpu_available FROM peer_nodes;"
```

Check rewards are being created:
```bash
sqlite3 dev_assets/db.sqlite "SELECT peer_node_id, reward_type, base_amount, multiplier, final_amount, status FROM peer_rewards ORDER BY created_at DESC LIMIT 5;"
```

---

## Phase 2: Mac Studio Peer Connection

### On Mac Studio

```bash
cd pcg-cc-mcp
git pull origin new
cargo build --release -p alpha-protocol-core
./start-peer-node.sh
```

### Verify Connection

Check Mac logs:
```bash
tail -f /tmp/apn_peer.log
```

Should see:
```
✅ Connected to NATS relay at nats://nonlocal.info:4222
✅ Mesh peer ID: apn_918ad440
📡 Publishing heartbeat every 30 seconds
```

### On Master (Pythia)

Check master sees Mac peer:
```bash
sqlite3 dev_assets/db.sqlite "SELECT node_id, wallet_address, cpu_cores, ram_mb, storage_gb FROM peer_nodes;"
```

Should show both:
- apn_814d37f4 (master)
- apn_918ad440 (Mac Studio)

---

## Phase 3: API Testing

### Test Peer Balance API

**Master Node Balance:**
```bash
curl http://localhost:58297/api/peers/0x814d37f4ab0c7ef2d9c7aa5f3de9fc5c0f79b3b576b77f84edc9e773eed71c92/balance | jq
```

Expected response:
```json
{
  "success": true,
  "data": {
    "node_id": "apn_814d37f4",
    "wallet_address": "0x814d37f4...",
    "pending_vibe": 12000000,
    "distributed_vibe": 0,
    "confirmed_vibe": 0,
    "total_earned": 12000000,
    "reward_count": 120,
    "pending_usd": 1.20,
    "total_earned_usd": 1.20
  }
}
```

**Mac Studio Balance:**
```bash
curl http://localhost:58297/api/peers/0x918ad440fd6c0e599b8b6deae44a38aae80ec2da145848ae904ad97c8fe436d4/balance | jq
```

### Test Rewards List

```bash
curl "http://localhost:58297/api/peers/0x814d37f4ab0c7ef2d9c7aa5f3de9fc5c0f79b3b576b77f84edc9e773eed71c92/rewards?limit=10" | jq
```

### Test Network Stats

```bash
curl http://localhost:58297/api/network/stats | jq
```

Expected:
```json
{
  "success": true,
  "data": {
    "total_peers": 2,
    "active_peers": 2,
    "total_rewards_distributed": 0,
    "total_pending_rewards": 24000000,
    "total_batches": 0
  }
}
```

### Test All Peers List

```bash
curl http://localhost:58297/api/peers | jq
```

---

## Phase 4: Reward Distribution Test

### Let Rewards Accumulate

Wait 5-10 minutes for rewards to accumulate. Monitor:

```bash
# Watch rewards being created
watch -n 30 "sqlite3 dev_assets/db.sqlite 'SELECT COUNT(*), SUM(final_amount) FROM peer_rewards WHERE status=\"pending\";'"
```

### Check Pending Rewards

```bash
sqlite3 dev_assets/db.sqlite "SELECT node_id, SUM(final_amount) as total_pending FROM peer_rewards pr JOIN peer_nodes pn ON pn.id = pr.peer_node_id WHERE pr.status = 'pending' GROUP BY node_id;"
```

### Trigger Distribution (Manual)

If distributor is running, it will auto-distribute every 5 minutes. Check logs:

```bash
tail -f /tmp/apn_distributor.log | grep "Batch\|Distribution"
```

Expected:
```
📦 Found 240 pending rewards to distribute
🎁 Created batch #1 with 2 peers, total 240 VIBE
🚀 Sending batch to Aptos blockchain...
💸 Sending 120 VIBE to 0x814d37f4...
✅ Transaction submitted: 0x...
💸 Sending 120 VIBE to 0x918ad440...
✅ Transaction submitted: 0x...
🎉 Batch distribution complete!
```

### Verify Batch Created

```bash
sqlite3 dev_assets/db.sqlite "SELECT batch_number, total_rewards, total_amount, status, aptos_tx_hash FROM reward_batches;"
```

### Verify Rewards Updated

```bash
sqlite3 dev_assets/db.sqlite "SELECT status, COUNT(*) FROM peer_rewards GROUP BY status;"
```

Should show:
- pending: 0
- distributed: 240
- confirmed: 0 (will be confirmed after blockchain check)

---

## Phase 5: Blockchain Verification

### Check Aptos Transactions

For each transaction hash from the batch, verify on Aptos Explorer:

```
https://explorer.aptoslabs.com/txn/[TX_HASH]?network=testnet
```

### Check Peer Wallet Balances

**Master Node Wallet:**
```
https://explorer.aptoslabs.com/account/0x814d37f4ab0c7ef2d9c7aa5f3de9fc5c0f79b3b576b77f84edc9e773eed71c92?network=testnet
```

**Mac Studio Wallet:**
```
https://explorer.aptoslabs.com/account/0x918ad440fd6c0e599b8b6deae44a38aae80ec2da145848ae904ad97c8fe436d4?network=testnet
```

Verify VIBE balance increased!

---

## Phase 6: End-to-End Validation

### Test Complete Flow

1. **Heartbeats Active** ✓
   - Both nodes sending heartbeats every 30s
   - Master node logs show incoming heartbeats

2. **Rewards Calculated** ✓
   - Reward tracker creating reward records every 60s
   - Multipliers applied correctly (GPU = 2x)

3. **Batches Created** ✓
   - Distributor grouping rewards every 5 minutes
   - Batch records in database

4. **Tokens Distributed** ✓
   - Aptos transactions submitted
   - Transaction hashes recorded

5. **Wallets Funded** ✓
   - Peer wallets show increased VIBE balance
   - Balances match reward amounts

6. **API Working** ✓
   - Balance endpoints return correct data
   - USD values calculated properly
   - Network stats accurate

---

## Expected Earnings Timeline

### After 1 Hour

**Master Node (24 cores, RTX 3080 Ti):**
- Heartbeats: 120 (every 30s)
- Base reward: 12 VIBE (0.1 × 120)
- GPU multiplier: 2x
- Total: **24 VIBE** (~$0.24)

**Mac Studio (20 cores, 64GB RAM):**
- Heartbeats: 120
- Base reward: 12 VIBE
- No multipliers
- Total: **12 VIBE** (~$0.12)

**Network Total: 36 VIBE/hour** (~$0.36/hour)

### After 24 Hours

- Master: **576 VIBE** (~$5.76)
- Mac Studio: **288 VIBE** (~$2.88)
- **Network: 864 VIBE/day** (~$8.64/day)

### After 30 Days

- Master: **~17,280 VIBE** (~$172.80)
- Mac Studio: **~8,640 VIBE** (~$86.40)
- **Network: ~25,920 VIBE/month** (~$259.20/month)

---

## Troubleshooting

### No Rewards Being Created

**Check:**
1. Reward tracker service is running
2. NATS relay connection active
3. Heartbeats being received
4. Database migration applied

**Debug:**
```bash
# Check if heartbeats are coming in
sqlite3 dev_assets/db.sqlite "SELECT node_id, last_heartbeat_at FROM peer_nodes;"

# Check reward tracker logs
grep "RewardTracker\|💰" /tmp/apn_node.log
```

### Distribution Not Happening

**Check:**
1. Distributor service is running
2. Rewards wallet loaded correctly
3. Minimum distribution amount met
4. No errors in distributor logs

**Debug:**
```bash
# Check pending rewards amount
sqlite3 dev_assets/db.sqlite "SELECT SUM(final_amount) FROM peer_rewards WHERE status='pending';"

# Should be > 100000000 (1 VIBE minimum)
```

### API Returning 404

**Check:**
1. Wallet address is correct (0x prefix)
2. Peer exists in database
3. Server is running on port 58297

**Debug:**
```bash
# Test health endpoint
curl http://localhost:58297/health

# Check if peer exists
sqlite3 dev_assets/db.sqlite "SELECT * FROM peer_nodes WHERE wallet_address LIKE '%918ad440%';"
```

---

## Monitoring Commands

### Real-Time Monitoring

**Watch pending rewards accumulate:**
```bash
watch -n 10 "curl -s http://localhost:58297/api/network/stats | jq '.data'"
```

**Monitor heartbeats:**
```bash
tail -f /tmp/apn_node.log | grep --line-buffered "Heartbeat\|Reward"
```

**Watch distribution:**
```bash
tail -f /tmp/apn_distributor.log | grep --line-buffered "Batch\|Distribution\|Transaction"
```

### Database Queries

**Peer summary:**
```sql
SELECT
    pn.node_id,
    pn.gpu_available,
    COUNT(pr.id) as reward_count,
    SUM(pr.final_amount) as total_earned,
    SUM(CASE WHEN pr.status = 'pending' THEN pr.final_amount ELSE 0 END) as pending,
    SUM(CASE WHEN pr.status = 'confirmed' THEN pr.final_amount ELSE 0 END) as confirmed
FROM peer_nodes pn
LEFT JOIN peer_rewards pr ON pr.peer_node_id = pn.id
GROUP BY pn.node_id;
```

**Recent rewards:**
```sql
SELECT
    datetime(pr.created_at) as time,
    pn.node_id,
    pr.reward_type,
    pr.base_amount,
    pr.multiplier,
    pr.final_amount,
    pr.status
FROM peer_rewards pr
JOIN peer_nodes pn ON pn.id = pr.peer_node_id
ORDER BY pr.created_at DESC
LIMIT 20;
```

---

## Success Criteria

✅ **Heartbeat Tracking:**
- Both nodes appear in peer_nodes table
- Last heartbeat updates every 30s
- Rewards created every 60s

✅ **Reward Calculation:**
- Base amount: 10000000 (0.1 VIBE)
- GPU multiplier: 2.0 for nodes with GPU
- Final amounts calculated correctly

✅ **Batch Distribution:**
- Batches created every 5 minutes
- Rewards grouped by wallet
- Status updated: pending → batched → distributed

✅ **Blockchain Integration:**
- Transactions submitted to Aptos
- Transaction hashes recorded
- Peer wallets receive VIBE tokens

✅ **API Endpoints:**
- Balance endpoints return accurate data
- USD calculations correct ($0.01/VIBE)
- Network stats match database

---

## 🎉 Testing Complete!

Once all phases pass, the peer reward distribution system is fully operational!

**Next Steps:**
1. Implement actual Aptos token transfers (replace simulation)
2. Add blockchain confirmation watcher
3. Implement retry logic for failed distributions
4. Add admin dashboard for reward management
5. Scale testing with more peer nodes

**Production Checklist:**
- [ ] Fund rewards wallet with 110M VIBE
- [ ] Enable reward distributor service
- [ ] Monitor for 24 hours
- [ ] Verify first batch distribution
- [ ] Confirm tokens received on-chain
- [ ] Announce to network participants!

---

**Questions or Issues?**
Check logs in `/tmp/apn_*.log` or contact the development team.

🏴 **Welcome to the Sovereign Economy!** 💰
