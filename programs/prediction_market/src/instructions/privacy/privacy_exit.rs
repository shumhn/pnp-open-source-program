//! Privacy Exit Pipeline
//!
//! This file contains the complete modular pipeline for exiting a Dark Pool position
//! or converting a public position into a shielded payout.
//!
//! Step 1: InitPrivacyClaim - Pre-creates the payout PDA and its collateral vault.
//! Step 2: Redeem - Either `redeem_privacy` (public) or `redeem_privacy_position` (dark pool).
//! Step 3: ClaimPrivacy - Revealing the secret and releasing funds to an unlinked wallet.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface, transfer_checked, burn, Burn, TransferChecked},
};
use anchor_lang::solana_program::keccak;

use crate::state::{Config, Market, MarketStatus, Outcome, PrivacyClaim, PrivacyPosition};

// =============================================================================
// STEP 1: INITIALIZE PRIVACY CLAIM
// =============================================================================

#[derive(Accounts)]
#[instruction(commitment: [u8; 32])]
pub struct InitPrivacyClaim<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub market: Box<Account<'info, Market>>,

    #[account(
        init,
        payer = user,
        space = 8 + PrivacyClaim::INIT_SPACE,
        seeds = [PrivacyClaim::SEED, market.key().as_ref(), commitment.as_ref()],
        bump
    )]
    pub privacy_claim: Box<Account<'info, PrivacyClaim>>,

    pub collateral_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = user,
        associated_token::mint = collateral_mint,
        associated_token::authority = privacy_claim,
    )]
    pub privacy_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitPrivacyClaim<'info> {
    pub fn init_privacy_claim(&mut self, commitment: [u8; 32], bump: u8) -> Result<()> {
        let claim = &mut self.privacy_claim;
        claim.market = self.market.key();
        claim.mint = self.collateral_mint.key();
        claim.amount = 0;
        claim.commitment = commitment;
        claim.lock_until = 0;
        claim.redeemed = false;
        claim.nonce = 0;
        claim.bump = bump;
        Ok(())
    }
}

// =============================================================================
// STEP 2A: REDEEM PRIVACY (PUBLIC -> PRIVATE)
// =============================================================================

#[derive(Accounts)]
#[instruction(commitment: [u8; 32])]
pub struct RedeemPrivacy<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = market.status == MarketStatus::Resolved @ PrivacyError::NotResolved,
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(seeds = [Config::SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,

    #[account(
        mut,
        seeds = [PrivacyClaim::SEED, market.key().as_ref(), commitment.as_ref()],
        bump = privacy_claim.bump,
    )]
    pub privacy_claim: Box<Account<'info, PrivacyClaim>>,

    #[account(mut)]
    pub yes_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub no_mint: Box<InterfaceAccount<'info, Mint>>,

    pub collateral_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut, associated_token::mint = yes_mint, associated_token::authority = user)]
    pub user_yes: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, associated_token::mint = no_mint, associated_token::authority = user)]
    pub user_no: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, associated_token::mint = collateral_mint, associated_token::authority = market)]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, associated_token::mint = collateral_mint, associated_token::authority = privacy_claim)]
    pub privacy_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> RedeemPrivacy<'info> {
    pub fn redeem_privacy(&mut self, commitment: [u8; 32]) -> Result<()> {
        let market = &mut self.market;
        let (user_balance, total_supply, winning_mint, user_account) = match market.outcome {
            Outcome::Yes => (self.user_yes.amount, market.yes_supply, self.yes_mint.to_account_info(), self.user_yes.to_account_info()),
            Outcome::No => (self.user_no.amount, market.no_supply, self.no_mint.to_account_info(), self.user_no.to_account_info()),
            Outcome::Undetermined => return err!(PrivacyError::NotResolved),
        };

        require!(user_balance > 0, PrivacyError::NoWinningTokens);

        let raw_collateral = (user_balance as u128).checked_mul(market.reserves as u128).unwrap().checked_div(total_supply as u128).unwrap() as u64;
        let denomination = 1_000_000; 
        let collateral_to_lock = (raw_collateral / denomination) * denomination;
        require!(collateral_to_lock > 0, PrivacyError::AmountTooSmall);

        let tokens_to_burn = (collateral_to_lock as u128).checked_mul(total_supply as u128).unwrap().checked_div(market.reserves as u128).unwrap() as u64;

        burn(CpiContext::new(self.token_program.to_account_info(), Burn { mint: winning_mint, from: user_account, authority: self.user.to_account_info() }), tokens_to_burn)?;

        let clock = Clock::get()?;
        self.privacy_claim.amount = collateral_to_lock;
        self.privacy_claim.commitment = commitment;
        self.privacy_claim.lock_until = clock.unix_timestamp + 5;

        let config_key = self.config.key();
        let market_id_bytes = market.id.to_le_bytes();
        let market_seeds = &[crate::state::market::Market::SEED, config_key.as_ref(), &market_id_bytes, &[market.bump]];
        let market_signer = &[&market_seeds[..]];

        transfer_checked(CpiContext::new_with_signer(self.token_program.to_account_info(), TransferChecked { from: self.vault.to_account_info(), mint: self.collateral_mint.to_account_info(), to: self.privacy_vault.to_account_info(), authority: market.to_account_info() }, market_signer), collateral_to_lock, self.collateral_mint.decimals)?;

        market.reserves -= collateral_to_lock;
        if market.outcome == Outcome::Yes { market.yes_supply -= tokens_to_burn; } else { market.no_supply -= tokens_to_burn; }

        emit!(PrivacyClaimCreated { market_id: market.id, commitment, amount: collateral_to_lock });
        Ok(())
    }
}

// =============================================================================
// STEP 2B: REDEEM PRIVACY POSITION (DARK POOL -> PRIVATE PAYOUT)
// =============================================================================

#[derive(Accounts)]
#[instruction(position_commitment: [u8; 32], payout_commitment: [u8; 32])]
pub struct RedeemPrivacyPosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, constraint = market.status == MarketStatus::Resolved @ PrivacyError::NotResolved)]
    pub market: Box<Account<'info, Market>>,

    #[account(seeds = [Config::SEED], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,

    #[account(mut, seeds = [PrivacyPosition::SEED, market.key().as_ref(), position_commitment.as_ref()], bump = privacy_position.bump)]
    pub privacy_position: Box<Account<'info, PrivacyPosition>>,

    #[account(mut, seeds = [PrivacyClaim::SEED, market.key().as_ref(), payout_commitment.as_ref()], bump = privacy_claim.bump)]
    pub privacy_claim: Box<Account<'info, PrivacyClaim>>,

    #[account(mut)]
    pub yes_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub no_mint: Box<InterfaceAccount<'info, Mint>>,

    pub collateral_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut, associated_token::mint = yes_mint, associated_token::authority = privacy_position)]
    pub privacy_yes: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, associated_token::mint = no_mint, associated_token::authority = privacy_position)]
    pub privacy_no: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, associated_token::mint = collateral_mint, associated_token::authority = market)]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, associated_token::mint = collateral_mint, associated_token::authority = privacy_claim)]
    pub privacy_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> RedeemPrivacyPosition<'info> {
    pub fn redeem_privacy_position(&mut self, position_commitment: [u8; 32], payout_commitment: [u8; 32]) -> Result<()> {
        let market = &mut self.market;
        let privacy_pos = &mut self.privacy_position;
        let privacy_claim = &mut self.privacy_claim;

        let (pos_balance, total_supply, winning_mint, source_vault) = match market.outcome {
            Outcome::Yes => (privacy_pos.yes_amount, market.yes_supply, self.yes_mint.to_account_info(), self.privacy_yes.to_account_info()),
            Outcome::No => (privacy_pos.no_amount, market.no_supply, self.no_mint.to_account_info(), self.privacy_no.to_account_info()),
            Outcome::Undetermined => return err!(PrivacyError::NotResolved),
        };

        require!(pos_balance > 0, PrivacyError::NoWinningTokens);

        let raw_collateral = (pos_balance as u128).checked_mul(market.reserves as u128).unwrap().checked_div(total_supply as u128).unwrap() as u64;
        let denomination = 1_000_000; 
        let collateral_to_lock = (raw_collateral / denomination) * denomination;
        require!(collateral_to_lock > 0, PrivacyError::AmountTooSmall);

        let tokens_to_burn = (collateral_to_lock as u128).checked_mul(total_supply as u128).unwrap().checked_div(market.reserves as u128).unwrap() as u64;

        let market_key = market.key();
        let pos_seeds = &[PrivacyPosition::SEED, market_key.as_ref(), position_commitment.as_ref(), &[privacy_pos.bump]];
        let pos_signer = &[&pos_seeds[..]];

        burn(CpiContext::new_with_signer(self.token_program.to_account_info(), Burn { mint: winning_mint, from: source_vault, authority: privacy_pos.to_account_info() }, pos_signer), tokens_to_burn)?;

        let clock = Clock::get()?;
        privacy_claim.amount = collateral_to_lock;
        privacy_claim.commitment = payout_commitment;
        privacy_claim.lock_until = clock.unix_timestamp + 5;

        let config_key = self.config.key();
        let market_id_bytes = market.id.to_le_bytes();
        let market_seeds = &[crate::state::market::Market::SEED, config_key.as_ref(), &market_id_bytes, &[market.bump]];
        let market_signer = &[&market_seeds[..]];

        transfer_checked(CpiContext::new_with_signer(self.token_program.to_account_info(), TransferChecked { from: self.vault.to_account_info(), mint: self.collateral_mint.to_account_info(), to: self.privacy_vault.to_account_info(), authority: market.to_account_info() }, market_signer), collateral_to_lock, self.collateral_mint.decimals)?;

        market.reserves -= collateral_to_lock;
        if market.outcome == Outcome::Yes { market.yes_supply -= tokens_to_burn; privacy_pos.yes_amount -= tokens_to_burn; } else { market.no_supply -= tokens_to_burn; privacy_pos.no_amount -= tokens_to_burn; }

        emit!(PrivacyClaimCreated { market_id: market.id, commitment: payout_commitment, amount: collateral_to_lock });
        Ok(())
    }
}

// =============================================================================
// STEP 3: CLAIM PRIVACY (FINAL PAYOUT)
// =============================================================================

#[derive(Accounts)]
#[instruction(secret: [u8; 32], commitment: [u8; 32])]
pub struct ClaimPrivacy<'info> {
    #[account(mut)]
    pub claimant: Signer<'info>,

    #[account(
        mut,
        seeds = [PrivacyClaim::SEED, privacy_claim.market.as_ref(), commitment.as_ref()],
        bump = privacy_claim.bump,
        constraint = privacy_claim.commitment == commitment @ PrivacyError::InvalidReveal,
        constraint = !privacy_claim.redeemed @ PrivacyError::AlreadyRedeemed,
    )]
    pub privacy_claim: Account<'info, PrivacyClaim>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,

    #[account(mut, associated_token::mint = collateral_mint, associated_token::authority = privacy_claim)]
    pub privacy_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(init_if_needed, payer = claimant, associated_token::mint = collateral_mint, associated_token::authority = recipient_account)]
    pub recipient_collateral: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Validated cryptographically via keccak-256
    pub recipient_account: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> ClaimPrivacy<'info> {
    pub fn claim(&mut self, secret: [u8; 32]) -> Result<()> {
        let privacy_claim = &mut self.privacy_claim;
        let recipient = self.recipient_account.key();
        let clock = Clock::get()?;

        require!(clock.unix_timestamp >= privacy_claim.lock_until, PrivacyError::StillLocked);

        let mut data = Vec::with_capacity(72);
        data.extend_from_slice(&secret);
        data.extend_from_slice(recipient.as_ref());
        data.extend_from_slice(&privacy_claim.nonce.to_le_bytes());
        
        let reveal_hash = keccak::hash(&data).0;
        require!(reveal_hash == privacy_claim.commitment, PrivacyError::InvalidReveal);

        let privacy_seeds = &[PrivacyClaim::SEED, privacy_claim.market.as_ref(), privacy_claim.commitment.as_ref(), &[privacy_claim.bump]];
        let privacy_signer = &[&privacy_seeds[..]];

        transfer_checked(CpiContext::new_with_signer(self.token_program.to_account_info(), TransferChecked { from: self.privacy_vault.to_account_info(), mint: self.collateral_mint.to_account_info(), to: self.recipient_collateral.to_account_info(), authority: privacy_claim.to_account_info() }, privacy_signer), privacy_claim.amount, self.collateral_mint.decimals)?;

        privacy_claim.redeemed = true;
        emit!(PrivacyClaimRevealed { commitment: privacy_claim.commitment, recipient, amount: privacy_claim.amount });
        Ok(())
    }
}

// =============================================================================
// EVENTS & ERRORS
// =============================================================================

#[event]
pub struct PrivacyClaimCreated {
    pub market_id: u64,
    pub commitment: [u8; 32],
    pub amount: u64,
}

#[event]
pub struct PrivacyClaimRevealed {
    pub commitment: [u8; 32],
    pub recipient: Pubkey,
    pub amount: u64,
}

#[error_code]
pub enum PrivacyError {
    #[msg("Market is not resolved")]
    NotResolved,
    #[msg("No winning tokens to redeem")]
    NoWinningTokens,
    #[msg("Invalid secret or recipient reveal")]
    InvalidReveal,
    #[msg("Claim already redeemed")]
    AlreadyRedeemed,
    #[msg("Winning amount too small for fixed denomination")]
    AmountTooSmall,
    #[msg("Privacy lock period not yet expired")]
    StillLocked,
}
