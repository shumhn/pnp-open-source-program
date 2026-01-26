//! Global Protocol Configuration
//!
//! This account stores protocol-wide settings that apply to all markets.

use anchor_lang::prelude::*;

/// Global configuration account (singleton PDA)
///
/// Seeds: ["config"]
#[account]
#[derive(InitSpace)]
pub struct Config {
    /// Protocol administrator with special privileges
    pub admin: Pubkey,

    /// Oracle/AI address authorized to resolve markets
    /// This could be:
    /// - A multisig
    /// - An AI agent's wallet
    /// - A decentralized oracle network
    pub oracle: Pubkey,

    /// Collateral token mint (e.g., USDC, SOL wrapped)
    pub collateral_mint: Pubkey,

    /// Protocol fee in basis points (100 = 1%, max 10000 = 100%)
    pub protocol_fee_bps: u64,

    /// Total markets created (used as incrementing ID)
    pub market_count: u64,

    /// Minimum liquidity required to create a market
    pub min_liquidity: u64,

    /// PDA bump seed
    pub bump: u8,

    /// Whether the protocol is paused
    pub paused: bool,
}

impl Config {
    pub const SEED: &'static [u8] = b"config_v7";
}
