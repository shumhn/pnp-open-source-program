# Building Permissionless Prediction Markets with Smart Contracts & AI

> An open-source guide to creating decentralized prediction markets on Solana

---

## 🎯 Overview

Prediction markets are financial instruments that allow participants to trade on the outcomes of future events. This repository demonstrates how to build **permissionless** prediction markets where:

- ✅ **Privacy-First** execution using FHE and ZK-Compression
- ✅ **Anyone** can create markets without permission
- ✅ **Anyone** can trade without intermediaries  
- ✅ **AI agents** or decentralized oracles resolve outcomes
- ✅ **Smart contracts** enforce all rules trustlessly

```
┌────────────────────────────────────────────────────────────────────┐
│                    PERMISSIONLESS "PRIVATE PNP" FLOW                │
│                                                                     │
│   User A                     Smart Contract                User B   │
│   ┌─────┐             ┌─────────────────────────┐         ┌─────┐  │
│   │Create│──"Will X?"▶│  Hybrid Market Logic    │◀"Shield"│Trade│  │
│   │Market│            │ ┌──────┐   ┌──────────┐ │ "Buy"   │     │  │
│   └─────┘             │ │Public│   │Confiden. │ │         └─────┘  │
│                       │ │ AMM  │   │Execution │ │                  │
│                       │ └──────┘   └──────────┘ │                  │
│                       └────────────┬────────────┘                  │
│                                    │                                │
│                              ┌─────▼─────┐                         │
│                              │  AI/Oracle │                         │
│                              │  Resolver  │                         │
│                              └───────────┘                         │
└────────────────────────────────────────────────────────────────────┘
```

---

## 🧠 Core Concepts

### 1. Binary Outcome Markets

Each market represents a yes/no question about a future event:

| Question | Possible Outcomes |
|----------|-------------------|
| "Will BTC reach $100k in 2025?" | YES or NO |
| "Will it rain in NYC tomorrow?" | YES or NO |
| "Will Team A win the championship?" | YES or NO |

### 2. Outcome Tokens

For each market, two token types are created:

- **YES Token**: Pays out if the predicted event occurs
- **NO Token**: Pays out if the predicted event does NOT occur

After resolution:
- Winning tokens can be redeemed for the prize pool
- Losing tokens become worthless

### 3. Pythagorean Automated Market Maker (AMM)

Instead of traditional order books, we use the **Pythagorean Bonding Curve** for price discovery:

```
┌─────────────────────────────────────────────────────────────────┐
│                  PYTHAGOREAN BONDING CURVE                       │
│                                                                  │
│   Core Invariant:   R = √(YES² + NO²)                           │
│                                                                  │
│   Where:                                                         │
│   • R = Total reserves (collateral locked in contract)          │
│   • YES = Supply of YES outcome tokens                          │
│   • NO = Supply of NO outcome tokens                            │
│                                                                  │
│   ┌─────────────────────────────────────┐                       │
│   │     NO ▲                            │                       │
│   │        │     ╭─────╮                │                       │
│   │        │   ╱       ╲  ← Curve       │                       │
│   │        │ ╱           ╲    (R=const) │                       │
│   │        │╱             ╲             │                       │
│   │        └───────────────▶ YES        │                       │
│   │                                      │                       │
│   │   All valid states lie on this curve │                       │
│   └─────────────────────────────────────┘                       │
│                                                                  │
│   Price Formulas:                                                │
│   • YES_price = YES / R                                         │
│   • NO_price  = NO / R                                          │
│   • YES_price² + NO_price² = 1   ← Always!                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Why Pythagorean specifically?**
- **Probability-native**: Prices naturally bounded 0-1, sum to ~1
- **Always liquid**: Trade any size at any price point
- **Fair pricing**: Marginal cost = token_supply / reserves
- **No impermanent loss**: Unlike xy=k AMMs, designed for binary outcomes

### 4. Price as Probability

Token prices naturally represent the market's collective belief about probability:

| YES Price | NO Price | Market Belief |
|-----------|----------|---------------|
| 0.80 | 0.20 | 80% chance YES occurs |
| 0.50 | 0.50 | Uncertain / 50-50 |
| 0.10 | 0.90 | 90% chance NO occurs |

---

## 🏗️ Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                      PREDICTION MARKET SYSTEM                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐         ┌──────────────────┐             │
│  │   Global Config  │         │   Hybrid Market  │             │
│  │   ─────────────  │         │   ────────────   │             │
│  │ • Admin          │         │ • Question       │             │
│  │ • Oracle         │         │ • End Time       │             │
│  │ • Fee Settings   │         │ • Public State   │             │
│  │ • Collateral     │         │ • Shielded State │             │
│  └──────────────────┘         │ • Status         │             │
│           │                    └────────┬─────────┘             │
│           │                             │                        │
│           ▼                             ▼                        │
│  ┌──────────────────────────────────────────────────┐          │
│  │                   INSTRUCTIONS                    │          │
│  │  ┌──────────┐ ┌──────────┐ ┌─────────┐ ┌──────┐ │          │
│  │  │Initialize│ │  Create  │ │  Trade  │ │Redeem│ │          │
│  │  │          │ │  Market  │ │ (Public/│ │      │ │          │
│  │  │          │ │          │ │ Private)│ │      │ │          │
│  │  └──────────┘ └──────────┘ └─────────┘ └──────┘ │          │
│  └──────────────────────────────────────────────────┘          │
│                           │                                      │
│                           ▼                                      │
│  ┌──────────────────────────────────────────────────┐          │
│  │               PRIVACY INFRASTRUCTURE              │          │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐ │          │
│  │  │Confidential│  │     ZK     │  │  Auditor   │ │          │
│  │  │ Execution  │  │Compression │  │  View Key  │ │          │
│  │  └────────────┘  └────────────┘  └────────────┘ │          │
│  └──────────────────────────────────────────────────┘          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Account Structure (Solana PDAs)

```rust
// Global configuration (one per protocol)
GlobalConfig {
    admin: Pubkey,           // Protocol administrator
    oracle: Pubkey,          // Authorized resolver
    collateral_mint: Pubkey, // e.g., USDC
    fee_bps: u64,            // Protocol fee
    market_count: u64,       // Auto-incrementing ID
}

// Individual market
Market {
    id: u64,
    question: String,
    end_time: u64,
    yes_mint: Pubkey,
    no_mint: Pubkey,
    reserves: u64,
    yes_supply: u64,
    no_supply: u64,
    status: MarketStatus,
    outcome: Outcome,
}
```

---

## 📝 Market Lifecycle

### Phase 1: Creation (Permissionless)

Anyone can create a market by:

```rust
create_market(
    question: "Will ETH flip BTC by 2025?",
    end_time: 1735689600,  // Unix timestamp
    initial_liquidity: 1000_000000,  // 1000 USDC
)
```

**What happens:**
1. Market PDA is created
2. YES and NO token mints are created
3. Initial liquidity deposited to vault
4. Creator receives equal YES and NO tokens
5. Market opens for trading

### Phase 2: Trading

Users can buy/sell outcome tokens:

```rust
// Bullish on outcome? Buy YES
buy_tokens(amount: 100, buy_yes: true, min_out: 95)

// Changed your mind? Sell back
sell_tokens(amount: 50, sell_yes: true, min_out: 45)
```

**Price dynamics:**
- Buying YES → YES price increases, NO price decreases
- Buying NO → NO price increases, YES price decreases
- Prices always sum to ≈ 1

### Phase 3: Resolution

After end_time, the oracle resolves the market:

```rust
// Only callable by authorized oracle
resolve_market(yes_wins: true)
```

**Oracle options:**
1. **AI Agent**: Autonomous resolver monitoring real-world events
2. **Multisig**: Committee of trusted parties
3. **Decentralized Oracle**: Chainlink, Pyth, UMA, etc.

### Phase 4: Redemption

Winners claim their share of the prize pool:

```rust
redeem()
// Burns winning tokens, receives proportional collateral
```

**Calculation:**
```
payout = (user_winning_tokens / total_winning_supply) × total_reserves
```

---

## 🤖 AI Integration for Resolution

### Why AI Oracles?

Traditional prediction markets face the "oracle problem" - who decides the outcome?

**AI Agents offer:**
- 24/7 monitoring of real-world events
- Scalable resolution for many markets
- Lower operational overhead
- Transparent decision-making

### Implementation Pattern

```typescript
// Example AI Oracle Agent (TypeScript/Node.js)
class AIMarketResolver {
    async checkAndResolve(marketId: number) {
        const market = await program.account.market.fetch(marketPDA);
        
        if (market.endTime < Date.now() / 1000) {
            // Query AI model or external data source
            const outcome = await this.determineOutcome(market.question);
            
            // Submit resolution transaction
            await program.methods
                .resolveMarket(outcome === 'yes')
                .accounts({ oracle: oracleKeypair.publicKey })
                .signers([oracleKeypair])
                .rpc();
        }
    }
    
    async determineOutcome(question: string): Promise<'yes' | 'no'> {
        // Integrate with:
        // - OpenAI/Anthropic for text analysis
        // - News APIs for event verification
        // - Price feeds for financial outcomes
        // - Sports APIs for game results
    }
}
```

### Hybrid Resolution Models

```
┌─────────────────────────────────────────────────────────┐
│              RESOLUTION STRATEGIES                       │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  1. PURE AI                                             │
│     AI agent autonomously resolves                       │
│     Best for: High-frequency, objective outcomes        │
│                                                          │
│  2. AI + HUMAN REVIEW                                   │
│     AI proposes, human committee approves               │
│     Best for: High-stakes or ambiguous markets          │
│                                                          │
│  3. OPTIMISTIC ORACLE (UMA-style)                       │
│     AI resolves, disputed outcomes go to vote           │
│     Best for: Decentralized, trustless resolution       │
│                                                          │
│  4. DATA FEED                                           │
│     Direct oracle feed (Pyth, Chainlink)                │
│     Best for: Price/numeric outcomes                    │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

---

## 💰 Economic Model

### Fee Structure

```
┌─────────────────────────────────────────┐
│           FEE DISTRIBUTION              │
├─────────────────────────────────────────┤
│                                          │
│  User trades 100 USDC                    │
│          │                               │
│          ▼                               │
│  ┌───────────────────┐                  │
│  │ Protocol Fee: 3%  │                  │
│  │ (3 USDC)          │                  │
│  └─────────┬─────────┘                  │
│            │                             │
│     ┌──────┴──────┐                     │
│     │             │                      │
│     ▼             ▼                      │
│  ┌──────┐    ┌──────────┐              │
│  │Admin │    │  Creator │              │
│  │ 90%  │    │   10%    │              │
│  │2.7USD│    │  0.3USD  │              │
│  └──────┘    └──────────┘              │
│                                          │
│  Market receives: 97 USDC               │
│                                          │
└─────────────────────────────────────────┘
```

### Revenue Opportunities

1. **Protocol Treasury**: Platform sustainability
2. **Creator Incentives**: Reward market creation
3. **Liquidity Mining**: Incentivize liquidity provision
4. **Governance Token**: Share protocol ownership

---

## 🔐 Security Considerations

### Smart Contract Security

| Risk | Mitigation |
|------|------------|
| Arithmetic overflow | Checked math operations |
| Unauthorized access | PDA-based access control |
| Reentrancy | Single-instruction atomicity |
| Front-running | Slippage protection |
| Oracle manipulation | Multiple data sources |

### Economic Security

| Risk | Mitigation |
|------|------------|
| Market manipulation | Bonding curve limits impact |
| Oracle collusion | Decentralized resolution |
| Flash loan attacks | Per-block limits |
| Liquidity drain | Minimum reserves |

---

## 🚀 Getting Started

### Prerequisites

```bash
# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install latest
avm use latest

# Install dependencies
yarn install
```

### Build & Deploy

```bash
# Build the program
anchor build

# Run tests
anchor test

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

### Create Your First Market

```typescript
import * as anchor from "@coral-xyz/anchor";
import { PredictionMarket } from "../target/types/prediction_market";

const program = anchor.workspace.PredictionMarket as Program<PredictionMarket>;

// Create a market
await program.methods
    .createMarket(
        "Will BTC reach $100k in 2025?",
        new anchor.BN(1735689600),  // End time
        new anchor.BN(1000_000000)  // 1000 USDC initial liquidity
    )
    .accounts({
        creator: wallet.publicKey,
        // ... other accounts
    })
    .rpc();
```

---

## 📚 Further Reading

### Technical Resources
- [Solana Cookbook](https://solanacookbook.com)
- [Anchor Documentation](https://www.anchor-lang.com)
- [SPL Token Program](https://spl.solana.com/token)

### Prediction Market Theory
- [Hanson's Market Scoring Rules](http://mason.gmu.edu/~rhanson/mktscore.pdf)
- [Logarithmic Market Scoring Rule](https://www.cs.cmu.edu/~sandholm/liquidity-sensitive%20automated%20market%20maker.teac.pdf)
- [Vitalik on Prediction Markets](https://vitalik.ca/general/2021/02/18/election.html)

### AI Oracle Research
- [Optimistic Oracles (UMA)](https://docs.uma.xyz/)
- [AI Agents in DeFi](https://arxiv.org/abs/2302.00000)

---

## 🤝 Contributing

This is an open-source skeleton implementation. Contributions welcome:

1. Fork the repository
2. Create a feature branch
3. Submit a pull request

---

## 📜 License

MIT License - See LICENSE file for details.

---

<div align="center">

**Built for the decentralized future 🌐**

*Permissionless • Trustless • Open*

</div>
