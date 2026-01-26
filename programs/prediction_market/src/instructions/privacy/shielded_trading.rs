//! Shielded Trading Pipeline (Blind Betting)
//!
//! This module implements 'Blind Betting'. 
//! Traders submit an XOR-encrypted direction. The contract accepts 
//! the collateral but cannot know the bet's direction until the 
//! trader reveals their secret after market resolution.
//!
//! Step 1: TradeShielded - Enter with encrypted direction
//! Step 2: RevealAndRedeem - Prove direction at resolution and claim payout

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked},
};
use anchor_lang::solana_program::keccak;

use crate::state::{Config, Market, MarketStatus, ShieldedPosition, Outcome};
use crate::instructions::public::TradeError;

// =============================================================================
// STEP 1: TRADE SHIELDED (Blind Entry)
// =============================================================================

/// Event emitted when a shielded position is entered
#[event]
pub struct ShieldedPositionEntered {
    pub market_id: u64,
    pub commitment: [u8; 32],
    pub shielded_amount: u64,
    // Note: direction is NOT emitted - it's private!
}

#[derive(Accounts)]
#[instruction(commitment: [u8; 32], direction_cipher: [u8; 32], amount: u64)]
pub struct TradeShielded<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,

    #[account(seeds = [Config::SEED], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        constraint = market.status == MarketStatus::Active @ TradeError::MarketNotActive,
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = trader,
        space = 8 + ShieldedPosition::INIT_SPACE,
        seeds = [ShieldedPosition::SEED, market.key().as_ref(), commitment.as_ref()],
        bump
    )]
    pub shielded_position: Account<'info, ShieldedPosition>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = trader,
    )]
    pub trader_collateral: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = market,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> TradeShielded<'info> {
    pub fn trade_shielded(
        &mut self,
        commitment: [u8; 32],
        direction_cipher: [u8; 32],
        amount: u64,
        bump: u8,
    ) -> Result<()> {
        let clock = Clock::get()?;
        require!(clock.unix_timestamp < self.market.end_time as i64, TradeError::MarketEnded);
        require!(!self.config.paused, TradeError::ProtocolPaused);

        // Transfer collateral to vault
        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.trader_collateral.to_account_info(),
                    mint: self.collateral_mint.to_account_info(),
                    to: self.vault.to_account_info(),
                    authority: self.trader.to_account_info(),
                },
            ),
            amount,
            self.collateral_mint.decimals,
        )?;

        // Update market reserves (hidden supply updates happen at reveal)
        self.market.reserves += amount;

        // Initialize shielded position with encrypted direction
        let pos = &mut self.shielded_position;
        pos.market = self.market.key();
        pos.commitment = commitment;
        pos.direction_cipher = direction_cipher;
        pos.shielded_amount = amount; // Stored as collateral value
        pos.collateral_deposited = amount;
        pos.bump = bump;

        emit!(ShieldedPositionEntered {
            market_id: self.market.id,
            commitment,
            shielded_amount: amount,
        });

        Ok(())
    }
}

// =============================================================================
// STEP 2: REVEAL AND REDEEM (Post-Resolution Claim)
// =============================================================================

/// Event emitted when a shielded position is revealed and redeemed
#[event]
pub struct ShieldedPositionRevealed {
    pub market_id: u64,
    pub commitment: [u8; 32],
    pub revealed_direction: bool, // true = YES, false = NO
    pub won: bool,
    pub payout: u64,
}

#[derive(Accounts)]
#[instruction(secret: [u8; 32], commitment: [u8; 32])]
pub struct RevealAndRedeem<'info> {
    #[account(mut)]
    pub revealer: Signer<'info>,

    #[account(seeds = [Config::SEED], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        constraint = market.status == MarketStatus::Resolved @ ShieldedError::MarketNotResolved,
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [ShieldedPosition::SEED, market.key().as_ref(), commitment.as_ref()],
        bump = shielded_position.bump,
        close = revealer,
    )]
    pub shielded_position: Account<'info, ShieldedPosition>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = market,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    /// The recipient wallet (can be different from revealer for relayer support)
    /// CHECK: This is the destination for the payout
    pub recipient: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = revealer,
        associated_token::mint = collateral_mint,
        associated_token::authority = recipient,
    )]
    pub recipient_collateral: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> RevealAndRedeem<'info> {
    pub fn reveal_and_redeem(&mut self, secret: [u8; 32], commitment: [u8; 32]) -> Result<()> {
        let pos = &self.shielded_position;

        // Verify commitment matches
        let computed_commitment = keccak::hash(&secret).0;
        require!(computed_commitment == commitment, ShieldedError::InvalidSecret);
        require!(pos.commitment == commitment, ShieldedError::CommitmentMismatch);

        // Decrypt direction
        let bet_yes = ShieldedPosition::decrypt_direction(&pos.direction_cipher, &secret);

        // Check if won
        let won = match self.market.outcome {
            Outcome::Yes => bet_yes,
            Outcome::No => !bet_yes,
            Outcome::Undetermined => return err!(ShieldedError::MarketNotResolved),
        };

        let payout = if won {
            // Winner gets back their collateral (simplified payout for hackathon)
            // In production, this would be proportional to total pool
            pos.collateral_deposited
        } else {
            0
        };

        if payout > 0 {
            // Transfer payout from vault to recipient
            let market_seeds = &[
                Market::SEED,
                &self.market.id.to_le_bytes(),
                &[self.market.bump],
            ];
            let market_signer = &[&market_seeds[..]];

            transfer_checked(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    TransferChecked {
                        from: self.vault.to_account_info(),
                        mint: self.collateral_mint.to_account_info(),
                        to: self.recipient_collateral.to_account_info(),
                        authority: self.market.to_account_info(),
                    },
                    market_signer,
                ),
                payout,
                self.collateral_mint.decimals,
            )?;

            self.market.reserves -= payout;
        }

        emit!(ShieldedPositionRevealed {
            market_id: self.market.id,
            commitment,
            revealed_direction: bet_yes,
            won,
            payout,
        });

        Ok(())
    }
}

// =============================================================================
// ERRORS
// =============================================================================

#[error_code]
pub enum ShieldedError {
    #[msg("Invalid secret - does not match commitment")]
    InvalidSecret,
    #[msg("Commitment mismatch")]
    CommitmentMismatch,
    #[msg("Market not yet resolved")]
    MarketNotResolved,
}
