//! # Private PNP: Hidden Prediction Markets
//!
//! A simple, professional prediction market on Solana with built-in privacy.
//!
//! ## Overview
//!
//! This project allows users to trade on future events without leaking 1. their choices, 
//! 2. their identities, or 3. their bank balances.
//!
//! ## How it works
//! - Inco FHE protects the choices and the market odds.
//! - Light Protocol ZK-Compression protects the user's identity and money.
//!

use anchor_lang::prelude::*;

pub mod amm;
pub mod instructions;
pub mod state;

pub use amm::*;
pub use instructions::*;

// Replace with your deployed program ID
declare_id!("8NeEkxgPMV5AnZ8o5ksjPhqsHwkWXdvGCGyHmEt6tJTn");

/// Main Private PNP program
#[program]
pub mod private_pnp {
    use super::*;

    /// Initialize the protocol with global configuration
    pub fn initialize(
        ctx: Context<Initialize>,
        protocol_fee: u64,
        oracle: Pubkey,
    ) -> Result<()> {
        ctx.accounts.initialize(protocol_fee, oracle, &ctx.bumps)
    }

    /// Create market state (Step 1)
    pub fn create_market_state(
        ctx: Context<CreateMarketState>,
        question: String,
        end_time: u64,
    ) -> Result<()> {
        ctx.accounts.create_market_state(question, end_time, &ctx.bumps)
    }

    /// Create YES/NO token mints (Step 2)
    pub fn create_market_mints(ctx: Context<CreateMarketMints>) -> Result<()> {
        ctx.accounts.create_market_mints()
    }

    /// Create market vaults and creator token accounts (Step 3)
    pub fn create_market_vaults(ctx: Context<CreateMarketVaults>) -> Result<()> {
        ctx.accounts.create_market_vaults()
    }

    /// Fund market with initial liquidity (Step 4)
    pub fn fund_market(ctx: Context<FundMarket>, initial_liquidity: u64) -> Result<()> {
        ctx.accounts.fund_market(initial_liquidity)
    }



    /// Step 1: Open a private position
    pub fn init_privacy_position(ctx: Context<InitPrivacyPosition>, commitment: [u8; 32]) -> Result<()> {
        ctx.accounts.init_privacy_position(commitment, ctx.bumps.privacy_position)
    }

    /// Step 2: Buy tokens privately
    pub fn trade_privacy(
        ctx: Context<TradePrivacy>,
        commitment: [u8; 32],
        amount: u64,
        buy_yes: bool,
    ) -> Result<()> {
        ctx.accounts.trade_privacy(commitment, amount, buy_yes)
    }

    /// Initialize a privacy payout claim (Step 1 of Dark Pool Exit)
    pub fn init_privacy_claim(ctx: Context<InitPrivacyClaim>, commitment: [u8; 32]) -> Result<()> {
        ctx.accounts.init_privacy_claim(commitment, ctx.bumps.privacy_claim)
    }

    /// Redeem a privacy position (Step 2 of Dark Pool Exit)
    pub fn redeem_privacy_position(
        ctx: Context<RedeemPrivacyPosition>,
        position_commitment: [u8; 32],
        payout_commitment: [u8; 32],
    ) -> Result<()> {
        ctx.accounts.redeem_privacy_position(position_commitment, payout_commitment)
    }

    /// Initialize trader tokens accounts (Standard AMM)
    pub fn init_trader_vaults(_ctx: Context<InitTraderVaults>) -> Result<()> {
        Ok(())
    }

    /// Trade with hidden choices (using Inco encryption)
    pub fn trade_shielded(
        ctx: Context<TradeShielded>,
        commitment: [u8; 32],
        direction_cipher: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.trade_shielded(commitment, direction_cipher, amount, ctx.bumps.shielded_position)
    }

    /// Reveal direction and redeem payout (post-resolution)
    pub fn reveal_and_redeem(
        ctx: Context<RevealAndRedeem>,
        secret: [u8; 32],
        commitment: [u8; 32],
    ) -> Result<()> {
        ctx.accounts.reveal_and_redeem(secret, commitment)
    }

    /// Advanced choice privacy (using Confidential Execution)
    pub fn trade_confidential(
        ctx: Context<TradeConfidential>,
        commitment: [u8; 32],
        encrypted_direction: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.trade_confidential(commitment, encrypted_direction, amount, ctx.bumps.confidential_position)
    }

    /// Advanced wallet privacy (using ZK-Compression)
    pub fn create_compressed_position(
        ctx: Context<CreateCompressedPosition>,
        ownership_commitment: [u8; 32],
        encrypted_direction: [u8; 32],
        amount: u64,
        compliance_commitment: [u8; 32],
        view_key_hash: [u8; 32],
        validity_proof: Vec<u8>,
    ) -> Result<()> {
        ctx.accounts.create_compressed_position(
            ownership_commitment,
            encrypted_direction,
            amount,
            compliance_commitment,
            view_key_hash,
            validity_proof
        )
    }

    /// Create a market with hidden odds (using Inco)
    pub fn create_encrypted_market(
        ctx: Context<CreateEncryptedMarket>,
        market_id: u64,
        inco_pubkey: [u8; 32],
        initial_encrypted_reserves: Vec<u8>,
    ) -> Result<()> {
        ctx.accounts.create_encrypted_market(market_id, inco_pubkey, initial_encrypted_reserves, ctx.bumps.encrypted_market)
    }

    /// Update reserves privately
    pub fn update_encrypted_reserves(
        ctx: Context<UpdateEncryptedReserves>,
        encrypted_delta: Vec<u8>,
        is_yes: bool,
    ) -> Result<()> {
        ctx.accounts.update_encrypted_reserves(encrypted_delta, is_yes)
    }

    /// Buy outcome tokens (YES or NO)
    pub fn buy_tokens(
        ctx: Context<Trade>,
        amount: u64,
        buy_yes: bool,
        min_tokens_out: u64,
    ) -> Result<u64> {
        ctx.accounts.buy_tokens(amount, buy_yes, min_tokens_out)
    }

    /// Sell outcome tokens back to the pool
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
    pub fn resolve_market(ctx: Context<ResolveMarket>, yes_wins: bool) -> Result<()> {
        ctx.accounts.resolve_market(yes_wins)
    }

    /// Redeem winning tokens for collateral
    pub fn redeem(ctx: Context<Redeem>) -> Result<u64> {
        ctx.accounts.redeem()
    }

    /// Step 1: Collect winnings privately
    pub fn redeem_privacy(ctx: Context<RedeemPrivacy>, commitment: [u8; 32]) -> Result<()> {
        ctx.accounts.redeem_privacy(commitment)
    }

    /// Step 2: Withdraw money to a fresh wallet
    pub fn claim_privacy(ctx: Context<ClaimPrivacy>, secret: [u8; 32], _commitment: [u8; 32]) -> Result<()> {
        ctx.accounts.claim(secret)
    }
}
