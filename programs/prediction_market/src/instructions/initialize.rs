//! Protocol Initialization
//!
//! Sets up the global configuration for the prediction market protocol.
//! This is typically called once during deployment.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::state::Config;

/// Accounts required for protocol initialization
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Protocol administrator (becomes the admin)
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Global configuration account (created)
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [Config::SEED],
        bump,
    )]
    pub config: Account<'info, Config>,

    /// Collateral token mint (e.g., USDC)
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    /// System program
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    /// Initialize the protocol configuration
    pub fn initialize(
        &mut self,
        protocol_fee_bps: u64,
        oracle: Pubkey,
        bumps: InitializeBumps,
    ) -> Result<()> {
        // Validate fee is reasonable (max 30%)
        require!(protocol_fee_bps <= 3000, InitializeError::FeeTooHigh);

        self.config.set_inner(Config {
            admin: self.admin.key(),
            oracle,
            collateral_mint: self.collateral_mint.key(),
            protocol_fee_bps,
            market_count: 0,
            min_liquidity: 1_000_000, // 1 token with 6 decimals
            bump: bumps.config,
            paused: false,
        });

        msg!("Protocol initialized!");
        msg!("Admin: {}", self.admin.key());
        msg!("Oracle: {}", oracle);
        msg!("Fee: {} bps", protocol_fee_bps);

        Ok(())
    }
}

#[error_code]
pub enum InitializeError {
    #[msg("Protocol fee cannot exceed 30%")]
    FeeTooHigh,
}
