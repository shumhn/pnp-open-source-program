//! Prediction Market State
//!
//! Each market represents a single yes/no prediction with its own liquidity pool.

use anchor_lang::prelude::*;

/// Individual prediction market account
///
/// Seeds: ["market", market_id.to_le_bytes()]
#[account]
#[derive(InitSpace)]
pub struct Market {
    /// Unique market identifier
    pub id: u64,

    /// Market creator's address
    pub creator: Pubkey,

    /// The prediction question
    /// Example: "Will ETH flip BTC by market cap in 2025?"
    #[max_len(256)]
    pub question: String,

    /// Unix timestamp when trading ends
    pub end_time: u64,

    /// Unix timestamp when market was created
    pub created_at: u64,

    /// YES token mint address
    pub yes_mint: Pubkey,

    /// NO token mint address
    pub no_mint: Pubkey,

    /// Collateral token mint address
    pub collateral_mint: Pubkey,

    /// Total collateral reserves in the pool
    pub reserves: u64,

    /// Total YES tokens minted
    pub yes_supply: u64,

    /// Total NO tokens minted
    pub no_supply: u64,

    /// Market resolution status
    pub status: MarketStatus,

    /// Winning outcome (only valid after resolution)
    pub outcome: Outcome,

    /// PDA bump seed
    pub bump: u8,
}

impl Market {
    pub const SEED: &'static [u8] = b"market";
}

/// Market lifecycle status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug, Default)]
pub enum MarketStatus {
    /// Market is open for trading
    #[default]
    Active,
    /// Trading ended, awaiting resolution
    Ended,
    /// Market has been resolved
    Resolved,
    /// Market was cancelled/voided
    Cancelled,
}

/// Prediction outcome
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug, Default)]
pub enum Outcome {
    /// Not yet determined
    #[default]
    Undetermined,
    /// YES outcome occurred
    Yes,
    /// NO outcome occurred
    No,
}
