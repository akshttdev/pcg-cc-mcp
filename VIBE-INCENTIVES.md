# 💰 VIBE Incentives - Sovereign Stack Economics

## What is VIBE?

**VIBE** is the native reward token of the Alpha Protocol Network. It's the economic layer that makes the sovereign stack sustainable and incentivizes participation.

```
Contribute Resources → Earn VIBE → Trade or Use Services
```

## How You Earn VIBE

### 1. Run an APN Peer Node
✅ **Passive Income** - Just by being connected to the mesh
- Heartbeat rewards: Earn for staying online
- Network availability: More uptime = more VIBE

### 2. Contribute Compute Power
✅ **Active Income** - Execute tasks for the network
- CPU cycles
- GPU compute
- Memory allocation
- Task completion bonuses

### 3. Provide Bandwidth
✅ **Network Income** - Help relay data through the mesh
- Data transfer
- NATS relay support
- LibP2P routing

### 4. Store Data
✅ **Storage Income** - Contribute disk space
- Distributed storage
- Redundancy rewards
- Long-term storage bonuses

## VIBE Economics

### Current Tokenomics
- **Total Supply:** 1,000,000,000 VIBE (1 Billion)
- **Current Price:** $0.01 USD
- **Market Cap:** $10,000,000 USD
- **Status:** LIVE and tradeable

### Distribution
- 40% - Network Rewards (for contributors like you!)
- 30% - Treasury (network development)
- 20% - Liquidity Pool
- 10% - Team & Advisors (vested)

## Your Peer Node Earnings

### Base Rewards
```bash
Heartbeat (every 30s):     0.1 VIBE
Hour of uptime:            12 VIBE
Day of uptime:             288 VIBE
Month of uptime:           ~8,640 VIBE ≈ $86.40/month
```

### Task Execution Bonuses
```bash
Simple task:               1-10 VIBE
Medium task:               10-50 VIBE
Complex task:              50-500 VIBE
AI Agent task:             100-1000 VIBE
```

### Resource Contribution Multipliers
```bash
CPU > 8 cores:             1.5x multiplier
RAM > 16GB:                1.3x multiplier
GPU available:             2x multiplier
SSD storage:               1.2x multiplier
```

**Mac Studio with M2 Ultra could earn: ~15,000 VIBE/month! ($150/month)**

## Tracking Your Earnings

### Check Your Balance
```bash
# Via API
curl http://localhost:58297/api/vibe/balance

# Via Dashboard
http://dashboard.powerclubglobal.com/vibe
```

### View Transaction History
```bash
# Recent transactions
curl http://localhost:58297/api/vibe/transactions

# Earnings breakdown
curl http://localhost:58297/api/vibe/earnings
```

### Real-Time Stats
```bash
# Current session earnings
curl http://localhost:58297/api/vibe/session

# Network-wide stats
curl http://192.168.1.77:8081/api/vibe/network
```

## Using VIBE

### 1. Trade on Markets
- **Primary Exchange:** https://vibe-token.vercel.app
- **DEX Trading:** Uniswap, PancakeSwap (coming soon)
- **Direct Swaps:** BTC, ETH, USDC

### 2. Pay for Premium Services
- Access to premium AI agents
- Priority task execution
- Enhanced storage limits
- API rate limit increases

### 3. Stake for Governance
- Vote on network upgrades
- Propose new features
- Earn staking rewards (5% APY)

### 4. Withdraw to Bitcoin
```bash
# Convert VIBE → BTC
curl -X POST http://localhost:58297/api/vibe/withdraw \
  -d '{"amount": 10000, "to": "bitcoin_address"}'
```

## Earning Optimization

### Maximize Your VIBE Earnings

**1. Keep Node Online 24/7**
```bash
# Use systemd to auto-restart
sudo systemctl enable apn-peer
```

**2. Allocate Maximum Resources**
```bash
# In .env
MAX_CPU_CONTRIBUTION=90
MAX_RAM_CONTRIBUTION=24
MAX_STORAGE_CONTRIBUTION=500
```

**3. Enable GPU Compute**
```bash
# For ComfyUI image generation tasks
ENABLE_GPU=true
GPU_MEMORY=12
```

**4. Join Resource Pools**
- Coordinate with other nodes
- Share bandwidth costs
- Pool rewards for stability

### ROI Calculator

**Mac Studio M2 Ultra (Example):**
```
Hardware: $4,000
Monthly earnings: 15,000 VIBE ≈ $150
Yearly earnings: 180,000 VIBE ≈ $1,800
ROI: ~2.2 years at current price

BUT:
- Price appreciation potential
- Task bonuses not included
- Network growth multipliers
- Storage/bandwidth income
- REALISTIC: 1-2 year ROI as network scales
```

## VIBE Wallet

### Your Peer Node Wallet

Each APN node has a built-in wallet:
```bash
# View your wallet address
cat /tmp/apn_peer.log | grep "Wallet Address"

# Or check via API
curl http://localhost:58297/api/vibe/wallet
```

### Backup Your Wallet

**CRITICAL:** Save your 12-word recovery phrase!
```bash
# During peer setup, you'll see:
🔐 Recovery Phrase: [12 words]
⚠️  SAVE THIS SECURELY - Cannot be recovered!
```

### Security Best Practices

✅ **DO:**
- Save recovery phrase offline (paper backup)
- Use hardware wallet for large amounts
- Enable 2FA on withdrawals
- Regular balance checks

❌ **DON'T:**
- Share recovery phrase with anyone
- Store phrase digitally unencrypted
- Keep all VIBE on hot wallet
- Ignore security updates

## Tax Considerations

### In the US
- VIBE earnings = taxable income
- Report at USD value when received
- Capital gains on appreciation
- Mining/staking income rules apply

**Consult your tax professional!**

## VIBE Roadmap

### Q1 2026 ✅ (NOW)
- Basic network rewards
- Manual withdrawals
- Dashboard tracking

### Q2 2026 📍
- Automatic payouts
- Mobile wallet app
- DeFi integrations

### Q3 2026
- Lightning Network support
- Instant VIBE → BTC swaps
- Cross-chain bridges

### Q4 2026
- VIBE staking v2
- Governance voting
- DAO treasury

## Real-World Example

**Sarah runs an APN peer on her gaming PC:**

```
Hardware: RTX 3080, 32GB RAM, 1TB SSD
Uptime: 18 hours/day (sleeps 6h)
Tasks completed: ~50/day

Monthly Earnings:
- Base rewards:        6,480 VIBE ($64.80)
- Task bonuses:        3,000 VIBE ($30.00)
- GPU multiplier (2x): 5,000 VIBE ($50.00)
- Resource bonus:      1,500 VIBE ($15.00)

Total: 15,980 VIBE ≈ $160/month

Her electricity cost: ~$12/month
Net profit: $148/month! 🚀

If VIBE reaches $0.10:
Same earnings = $1,600/month
Net profit: $1,588/month! 🚀🚀
```

## FAQ

**Q: When do I get paid?**
A: Real-time! VIBE accumulates in your wallet continuously.

**Q: Minimum withdrawal?**
A: 1,000 VIBE minimum for BTC withdrawals.

**Q: What if my node goes offline?**
A: No penalty, just stop earning. Come back anytime!

**Q: Can I run multiple nodes?**
A: Yes! Each node earns independently.

**Q: Is this legal?**
A: Yes, similar to Bitcoin mining. Check local regulations.

---

## 🏴 Start Earning Today!

```bash
./start-peer-node.sh
```

**Every second your node is online, you're earning VIBE!**

💰 **Current network-wide earnings: 1M+ VIBE distributed daily!**

Trade VIBE: https://vibe-token.vercel.app
View on Dashboard: http://dashboard.powerclubglobal.com

**Welcome to the sovereign economy!** 🏴💰
