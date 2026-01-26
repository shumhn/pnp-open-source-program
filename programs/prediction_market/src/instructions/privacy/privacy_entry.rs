//! Privacy Entry Pipeline
//!
//! This file contains the complete modular pipeline for entering a Dark Pool position.
//! Due to Solana's 4KB stack limit, the entry process is split into
//! 2 atomic steps:
//!
//! Step 1: InitPrivacyPosition - Pre-creates the Ghost PDA and token vaults.
//! Step 2: TradePrivacy - Executes the AMM trade into the Ghost vaults.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, MintTo, TokenAccount, TokenInterface, TransferChecked, mint_to, transfer_checked},
};

use crate::amm::PythagoreanCurve;
use crate::state::{Config, Market, MarketStatus, PrivacyPosition};
use crate::instructions::public::TradeError;

// =============================================================================
// STEP 1: INITIALIZE PRIVACY POSITION
// =============================================================================

#[derive(Accounts)]
#[instruction(commitment: [u8; 32])]
pub struct InitPrivacyPosition<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,

    pub market: Box<Account<'info, Market>>,

    #[account(
        init,
        payer = trader,
        space = 8 + PrivacyPosition::INIT_SPACE,
        seeds = [PrivacyPosition::SEED, market.key().as_ref(), commitment.as_ref()],
        bump
    )]
    pub privacy_position: Box<Account<'info, PrivacyPosition>>,

    pub yes_mint: Box<InterfaceAccount<'info, Mint>>,
    pub no_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = trader,
        associated_token::mint = yes_mint,
        associated_token::authority = privacy_position,
    )]
    pub privacy_yes: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = trader,
        associated_token::mint = no_mint,
        associated_token::authority = privacy_position,
    )]
    pub privacy_no: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitPrivacyPosition<'info> {
    pub fn init_privacy_position(&mut self, commitment: [u8; 32], bump: u8) -> Result<()> {
        let pos = &mut self.privacy_position;
        pos.market = self.market.key();
        pos.commitment = commitment;
        pos.yes_amount = 0;
        pos.no_amount = 0;
        pos.bump = bump;
        Ok(())
    }
}

// =============================================================================
// STEP 2: TRADE PRIVACY
// =============================================================================

/// Event emitted when a privacy position is entered
#[event]
pub struct PrivacyPositionEntered {
    pub market_id: u64,
    pub commitment: [u8; 32],
    pub yes_amount: u64,
    pub no_amount: u64,
}

#[derive(Accounts)]
#[instruction(commitment: [u8; 32], amount: u64, buy_yes: bool)]
pub struct TradePrivacy<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,

    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Box<Account<'info, Config>>,

    #[account(
        mut,
        constraint = market.status == MarketStatus::Active @ TradeError::MarketNotActive,
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        mut,
        seeds = [PrivacyPosition::SEED, market.key().as_ref(), commitment.as_ref()],
        bump = privacy_position.bump,
    )]
    pub privacy_position: Box<Account<'info, PrivacyPosition>>,

    #[account(mut)]
    pub yes_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub no_mint: Box<InterfaceAccount<'info, Mint>>,

    pub collateral_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = trader,
    )]
    pub trader_collateral: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = market,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = yes_mint,
        associated_token::authority = privacy_position,
    )]
    pub privacy_yes: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = no_mint,
        associated_token::authority = privacy_position,
    )]
    pub privacy_no: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> TradePrivacy<'info> {
    pub fn trade_privacy(
        &mut self,
        commitment: [u8; 32],
        amount: u64,
        buy_yes: bool,
    ) -> Result<()> {
        let market = &mut self.market;
        
        let tokens_to_mint = {
            let (target_supply, other_supply) = if buy_yes {
                (market.yes_supply, market.no_supply)
            } else {
                (market.no_supply, market.yes_supply)
            };
            PythagoreanCurve::get_tokens_to_mint(market.reserves, target_supply, other_supply, amount)?
        };

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

        let config_seeds = &[Config::SEED, &[self.config.bump]];
        let signer_seeds = &[&config_seeds[..]];
        let (target_mint, target_vault) = if buy_yes {
            (self.yes_mint.to_account_info(), self.privacy_yes.to_account_info())
        } else {
            (self.no_mint.to_account_info(), self.privacy_no.to_account_info())
        };

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: target_mint,
                    to: target_vault,
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            tokens_to_mint,
        )?;

        market.reserves += amount;
        if buy_yes {
            market.yes_supply += tokens_to_mint;
            self.privacy_position.yes_amount += tokens_to_mint;
        } else {
            market.no_supply += tokens_to_mint;
            self.privacy_position.no_amount += tokens_to_mint;
        }

        emit!(PrivacyPositionEntered {
            market_id: market.id,
            commitment,
            yes_amount: self.privacy_position.yes_amount,
            no_amount: self.privacy_position.no_amount,
        });

        Ok(())
    }
}
