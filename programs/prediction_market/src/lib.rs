//! # Permissionless Prediction Markets
//!
//! This is an open-source skeleton implementation demonstrating how prediction markets
//! can be built permissionlessly on Solana using smart contracts and AI oracles.
//!
//! ## Overview
//!
//! Prediction markets allow users to trade on the outcomes of future events.
//! This implementation uses:
//! - **Outcome Tokens**: YES/NO tokens representing each side of a prediction
//! - **Automated Market Maker**: Bonding curve for dynamic pricing
//! - **Permissionless Creation**: Anyone can create markets
//! - **AI/Oracle Resolution**: Flexible resolution mechanism
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    PREDICTION MARKET                         │
//! │                                                              │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
//! │  │   Collateral  │───▶│     AMM      │───▶│  YES/NO      │  │
//! │  │   (USDC/SOL)  │    │   Bonding    │    │   Tokens     │  │
//! │  └──────────────┘    │    Curve     │    └──────────────┘  │
//! │                       └──────────────┘                       │
//! │                              │                               │
//! │                              ▼                               │
//! │                    ┌──────────────┐                          │
//! │                    │   Oracle/AI   │                         │
//! │                    │  Resolution   │                         │
//! │                    └──────────────┘                          │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use anchor_lang::prelude::*;

pub mod amm;
pub mod instructions;
pub mod state;

pub use amm::*;
pub use instructions::*;
pub use state::*;

// Replace with your deployed program ID
declare_id!("11111111111111111111111111111111");

/// Main prediction market program
#[program]
pub mod prediction_market {
    use super::*;

    /// Initialize the protocol with global configuration
    ///
    /// This sets up:
    /// - Admin authority
    /// - Protocol fee structure
    /// - Collateral token (e.g., USDC)
    /// - Oracle/AI resolver address
    ///
    /// # Arguments
    /// * `protocol_fee` - Fee in basis points (100 = 1%)
    /// * `oracle` - Address authorized to resolve markets
    pub fn initialize(
        ctx: Context<Initialize>,
        protocol_fee: u64,
        oracle: Pubkey,
    ) -> Result<()> {
        ctx.accounts.initialize(protocol_fee, oracle, ctx.bumps)
    }

    /// Create a new prediction market
    ///
    /// Anyone can create a market permissionlessly by:
    /// 1. Defining a question/statement to predict
    /// 2. Setting an end time for trading
    /// 3. Providing initial liquidity
    ///
    /// # Arguments
    /// * `question` - The prediction question (e.g., "Will BTC reach $100k by 2025?")
    /// * `end_time` - Unix timestamp when trading ends
    /// * `initial_liquidity` - Collateral to seed the market
    pub fn create_market(
        ctx: Context<CreateMarket>,
        question: String,
        end_time: u64,
        initial_liquidity: u64,
    ) -> Result<()> {
        ctx.accounts
            .create_market(question, end_time, initial_liquidity, ctx.bumps)
    }

    /// Buy outcome tokens (YES or NO)
    ///
    /// Uses the AMM bonding curve to calculate token output.
    /// Price adjusts dynamically based on supply/demand.
    ///
    /// # Arguments
    /// * `amount` - Amount of collateral to spend
    /// * `buy_yes` - true = buy YES tokens, false = buy NO tokens
    /// * `min_tokens_out` - Slippage protection
    pub fn buy_tokens(
        ctx: Context<Trade>,
        amount: u64,
        buy_yes: bool,
        min_tokens_out: u64,
    ) -> Result<u64> {
        ctx.accounts.buy_tokens(amount, buy_yes, min_tokens_out)
    }

    /// Sell outcome tokens back to the pool
    ///
    /// # Arguments
    /// * `amount` - Amount of tokens to sell
    /// * `sell_yes` - true = sell YES tokens, false = sell NO tokens
    /// * `min_collateral_out` - Slippage protection
    pub fn sell_tokens(
        ctx: Context<Trade>,
        amount: u64,
        sell_yes: bool,
        min_collateral_out: u64,
    ) -> Result<u64> {
        ctx.accounts
            .sell_tokens(amount, sell_yes, min_collateral_out)
    }

    /// Resolve the market (oracle/AI only)
    ///
    /// Called by the authorized oracle or AI agent to determine
    /// the winning outcome after the market ends.
    ///
    /// # Arguments
    /// * `yes_wins` - true if YES outcome occurred, false if NO
    pub fn resolve_market(ctx: Context<ResolveMarket>, yes_wins: bool) -> Result<()> {
        ctx.accounts.resolve_market(yes_wins)
    }

    /// Redeem winning tokens for collateral
    ///
    /// After market resolution, holders of winning tokens can
    /// redeem them for their proportional share of the prize pool.
    pub fn redeem(ctx: Context<Redeem>) -> Result<u64> {
        ctx.accounts.redeem()
    }
}
