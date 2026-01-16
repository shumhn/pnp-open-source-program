# 🎯 Permissionless Prediction Markets on Solana

> An open-source skeleton for building decentralized prediction markets with smart contracts and AI oracles

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Solana](https://img.shields.io/badge/Solana-1.18-blue)](https://solana.com)
[![Anchor](https://img.shields.io/badge/Anchor-0.30-purple)](https://anchor-lang.com)

---

## What is This?

This repository provides a **minimal, educational skeleton** demonstrating how permissionless prediction markets work on Solana. It's designed to help developers understand and build their own prediction market protocols.

```
  ┌─────────────┐          ┌─────────────┐          ┌─────────────┐
  │   CREATE    │    ───▶  │    TRADE    │    ───▶  │   RESOLVE   │
  │   MARKET    │          │  YES / NO   │          │  & REDEEM   │
  └─────────────┘          └─────────────┘          └─────────────┘
       Anyone              Bonding Curve            AI / Oracle
```

## ✨ Features

- **Permissionless Market Creation** - Anyone can create a prediction market
- **Pythagorean AMM** - `R = √(YES² + NO²)` bonding curve for price discovery  
- **Flexible Resolution** - Designed for AI agents or oracle integration
- **Clean Architecture** - Well-documented, educational codebase

## 📁 Project Structure

```
open-source-skeleton/
├── programs/
│   └── prediction_market/
│       └── src/
│           ├── lib.rs              # Program entry point
│           ├── state/
│           │   ├── config.rs       # Global protocol configuration
│           │   └── market.rs       # Individual market state
│           ├── instructions/
│           │   ├── initialize.rs   # Protocol initialization
│           │   ├── create_market.rs# Permissionless market creation
│           │   ├── trade.rs        # Buy/sell tokens
│           │   ├── resolve.rs      # Oracle resolution
│           │   └── redeem.rs       # Claim winnings
│           └── amm/
│               └── bonding_curve.rs # Price calculation
├── Anchor.toml
├── Cargo.toml
├── README.md
└── PERMISSIONLESS_PREDICTION_MARKETS.md  # Detailed guide
```

## 🚀 Quick Start

### Prerequisites

```bash
# Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install latest && avm use latest
```

### Build

```bash
cd open-source-skeleton
anchor build
```

### Test

```bash
anchor test
```

### Deploy

```bash
# Update program ID in lib.rs and Anchor.toml first
anchor deploy --provider.cluster devnet
```

## 📖 Core Concepts

### How It Works

1. **Initialize Protocol**: Admin sets up global configuration (fee, oracle, collateral token)

2. **Create Market**: Anyone can create a YES/NO prediction market with:
   - A question ("Will X happen?")
   - End time for trading
   - Initial liquidity

3. **Trade**: Users buy/sell YES or NO tokens
   - Prices determined by bonding curve
   - Price represents implied probability
   - Always liquid

4. **Resolve**: After end time, oracle/AI determines outcome

5. **Redeem**: Winners exchange tokens for proportional share of prize pool

### The Pythagorean Bonding Curve

We use the **Pythagorean AMM invariant** for pricing:

```
              R = √(YES² + NO²)

  Where:
  • R   = Total collateral reserves
  • YES = YES token supply  
  • NO  = NO token supply

  Price formulas:
  • YES_price = YES / R
  • NO_price  = NO / R
```

**Why this works for prediction markets:**
- Prices naturally represent probabilities (bounded 0-1)
- `YES_price² + NO_price²  = 1` always holds
- At 50/50 odds: each price ≈ 0.707 (1/√2)
- Buying one side increases its price, decreases the other
- Always liquid - no counterparty needed

## 🤖 AI Oracle Integration

The skeleton is designed for easy AI/oracle integration:

```typescript
// Example: AI agent resolves markets
const aiResolver = async (market) => {
  const question = market.question;
  
  // Your AI/data source logic here
  const outcome = await analyzeOutcome(question);
  
  // Submit resolution
  await program.methods
    .resolveMarket(outcome === 'yes')
    .accounts({ oracle: oracleWallet })
    .rpc();
};
```

## ⚠️ Disclaimer

This is an **educational skeleton** and NOT production-ready. Before deploying to mainnet:

- [ ] Comprehensive security audit
- [ ] Formal verification of bonding curve math
- [ ] Economic modeling and stress testing
- [ ] Oracle decentralization strategy
- [ ] Regulatory compliance review

## 📚 Learn More

- **[PERMISSIONLESS_PREDICTION_MARKETS.md](./PERMISSIONLESS_PREDICTION_MARKETS.md)** - Deep dive into concepts and architecture

## 🤝 Contributing

Contributions welcome! Please read our contributing guidelines and submit PRs.

## 📜 License

MIT License - feel free to use, modify, and distribute.

---

<div align="center">

**Built with ❤️ for the open-source community**

[Report Bug](../../issues) · [Request Feature](../../issues)

</div>
