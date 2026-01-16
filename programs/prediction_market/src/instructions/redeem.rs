//! Position Redemption
//!
//! After a market is resolved, holders of winning tokens can redeem
//! them for their proportional share of the prize pool.
//!
//! ## Redemption Calculation
//!
//! ```text
//! user_share = (user_tokens / total_winning_tokens) * total_reserves
//! ```
//!
//! For example:
//! - User holds 100 YES tokens
//! - Total YES supply: 1000 tokens
//! - Total reserves: 5000 USDC
//! - If YES wins: user receives (100/1000) * 5000 = 500 USDC

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        Mint, TokenAccount, TokenInterface, TransferChecked, Burn,
        transfer_checked, burn,
    },
};

use crate::state::{Config, Market, MarketStatus, Outcome};

/// Event emitted when a position is redeemed
#[event]
pub struct PositionRedeemed {
    pub market_id: u64,
    pub redeemer: Pubkey,
    pub tokens_burned: u64,
    pub collateral_received: u64,
}

/// Accounts for redemption
#[derive(Accounts)]
pub struct Redeem<'info> {
    /// User redeeming their position
    #[account(mut)]
    pub user: Signer<'info>,

    /// Protocol configuration
    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    /// Resolved market
    #[account(
        mut,
        constraint = market.status == MarketStatus::Resolved @ RedeemError::NotResolved,
    )]
    pub market: Account<'info, Market>,

    /// YES token mint
    #[account(
        mut,
        constraint = yes_mint.key() == market.yes_mint,
    )]
    pub yes_mint: InterfaceAccount<'info, Mint>,

    /// NO token mint
    #[account(
        mut,
        constraint = no_mint.key() == market.no_mint,
    )]
    pub no_mint: InterfaceAccount<'info, Mint>,

    /// Collateral mint
    #[account(
        constraint = collateral_mint.key() == market.collateral_mint,
    )]
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    /// User's YES token account
    #[account(
        mut,
        associated_token::mint = yes_mint,
        associated_token::authority = user,
    )]
    pub user_yes: InterfaceAccount<'info, TokenAccount>,

    /// User's NO token account
    #[account(
        mut,
        associated_token::mint = no_mint,
        associated_token::authority = user,
    )]
    pub user_no: InterfaceAccount<'info, TokenAccount>,

    /// User's collateral account
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = user,
    )]
    pub user_collateral: InterfaceAccount<'info, TokenAccount>,

    /// Market's reserve vault
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = market,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    /// Token program
    pub token_program: Interface<'info, TokenInterface>,
    /// Associated token program
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// System program
    pub system_program: Program<'info, System>,
}

impl<'info> Redeem<'info> {
    /// Redeem winning tokens for collateral
    pub fn redeem(&mut self) -> Result<u64> {
        // Determine winning token and user's balance
        let (user_balance, total_supply, winning_mint, user_account) = match self.market.outcome {
            Outcome::Yes => (
                self.user_yes.amount,
                self.market.yes_supply,
                &self.yes_mint,
                &self.user_yes,
            ),
            Outcome::No => (
                self.user_no.amount,
                self.market.no_supply,
                &self.no_mint,
                &self.user_no,
            ),
            Outcome::Undetermined => return err!(RedeemError::NotResolved),
        };

        require!(user_balance > 0, RedeemError::NoWinningTokens);

        // Calculate proportional share of reserves
        // share = (user_balance / total_supply) * reserves
        let collateral_to_receive = (user_balance as u128)
            .checked_mul(self.market.reserves as u128)
            .unwrap()
            .checked_div(total_supply as u128)
            .unwrap() as u64;

        // Burn user's winning tokens
        burn(
            CpiContext::new(
                self.token_program.to_account_info(),
                Burn {
                    mint: winning_mint.to_account_info(),
                    from: user_account.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            user_balance,
        )?;

        // Transfer collateral to user
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
                    to: self.user_collateral.to_account_info(),
                    authority: self.market.to_account_info(),
                },
                market_signer,
            ),
            collateral_to_receive,
            self.collateral_mint.decimals,
        )?;

        // Update market reserves
        self.market.reserves = self.market.reserves
            .checked_sub(collateral_to_receive)
            .unwrap();

        emit!(PositionRedeemed {
            market_id: self.market.id,
            redeemer: self.user.key(),
            tokens_burned: user_balance,
            collateral_received: collateral_to_receive,
        });

        Ok(collateral_to_receive)
    }
}

#[error_code]
pub enum RedeemError {
    #[msg("Market is not resolved")]
    NotResolved,
    #[msg("No winning tokens to redeem")]
    NoWinningTokens,
}
