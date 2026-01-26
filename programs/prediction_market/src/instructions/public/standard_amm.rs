//! Standard AMM Trading & Redemption
//!
//! This file contains the logic for public, non-private trading
//! and standard winning token redemptions.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, MintTo, TokenAccount, TokenInterface, TransferChecked, Burn, mint_to, transfer_checked, burn},
};

use crate::amm::PythagoreanCurve;
use crate::state::{Config, Market, MarketStatus, Outcome};

// =============================================================================
// PUBLIC TRADING (AMM)
// =============================================================================

#[event]
pub struct TokensBought {
    pub market_id: u64,
    pub buyer: Pubkey,
    pub is_yes: bool,
    pub collateral_in: u64,
    pub tokens_out: u64,
}

#[event]
pub struct TokensSold {
    pub market_id: u64,
    pub seller: Pubkey,
    pub is_yes: bool,
    pub tokens_in: u64,
    pub collateral_out: u64,
}

#[derive(Accounts)]
pub struct InitTraderVaults<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,

    pub yes_mint: InterfaceAccount<'info, Mint>,
    pub no_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = trader,
        associated_token::mint = yes_mint,
        associated_token::authority = trader,
    )]
    pub trader_yes: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = trader,
        associated_token::mint = no_mint,
        associated_token::authority = trader,
    )]
    pub trader_no: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Trade<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,

    #[account(seeds = [Config::SEED], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, constraint = market.status == MarketStatus::Active @ TradeError::MarketNotActive)]
    pub market: Account<'info, Market>,

    #[account(mut, constraint = yes_mint.key() == market.yes_mint)]
    pub yes_mint: InterfaceAccount<'info, Mint>,

    #[account(mut, constraint = no_mint.key() == market.no_mint)]
    pub no_mint: InterfaceAccount<'info, Mint>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,

    #[account(mut, associated_token::mint = collateral_mint, associated_token::authority = trader)]
    pub trader_collateral: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, associated_token::mint = yes_mint, associated_token::authority = trader)]
    pub trader_yes: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, associated_token::mint = no_mint, associated_token::authority = trader)]
    pub trader_no: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, associated_token::mint = collateral_mint, associated_token::authority = market)]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Trade<'info> {
    pub fn buy_tokens(&mut self, amount: u64, buy_yes: bool, min_tokens_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        require!(clock.unix_timestamp < self.market.end_time as i64, TradeError::MarketEnded);
        require!(!self.config.paused, TradeError::ProtocolPaused);

        let fee = amount.checked_mul(self.config.protocol_fee_bps).unwrap().checked_div(10000).unwrap();
        let amount_after_fee = amount.checked_sub(fee).unwrap();

        let (target_supply, other_supply) = if buy_yes { (self.market.yes_supply, self.market.no_supply) } else { (self.market.no_supply, self.market.yes_supply) };
        let tokens_out = PythagoreanCurve::get_tokens_to_mint(self.market.reserves, target_supply, other_supply, amount_after_fee)?;

        require!(tokens_out >= min_tokens_out, TradeError::SlippageExceeded);

        transfer_checked(CpiContext::new(self.token_program.to_account_info(), TransferChecked { from: self.trader_collateral.to_account_info(), mint: self.collateral_mint.to_account_info(), to: self.vault.to_account_info(), authority: self.trader.to_account_info() }), amount_after_fee, self.collateral_mint.decimals)?;

        let config_seeds = &[Config::SEED, &[self.config.bump]];
        let signer_seeds = &[&config_seeds[..]];
        let (mint, destination) = if buy_yes { (&self.yes_mint, &self.trader_yes) } else { (&self.no_mint, &self.trader_no) };

        mint_to(CpiContext::new_with_signer(self.token_program.to_account_info(), MintTo { mint: mint.to_account_info(), to: destination.to_account_info(), authority: self.config.to_account_info() }, signer_seeds), tokens_out)?;

        self.market.reserves += amount_after_fee;
        if buy_yes { self.market.yes_supply += tokens_out; } else { self.market.no_supply += tokens_out; }

        emit!(TokensBought { market_id: self.market.id, buyer: self.trader.key(), is_yes: buy_yes, collateral_in: amount, tokens_out });
        Ok(tokens_out)
    }

    pub fn sell_tokens(&mut self, amount: u64, sell_yes: bool, min_collateral_out: u64) -> Result<u64> {
        let clock = Clock::get()?;
        require!(clock.unix_timestamp < self.market.end_time as i64, TradeError::MarketEnded);
        require!(!self.config.paused, TradeError::ProtocolPaused);

        let (target_supply, other_supply) = if sell_yes { (self.market.yes_supply, self.market.no_supply) } else { (self.market.no_supply, self.market.yes_supply) };
        let collateral_out = PythagoreanCurve::get_reserve_to_release(self.market.reserves, target_supply, other_supply, amount)?;

        let fee = collateral_out.checked_mul(self.config.protocol_fee_bps).unwrap().checked_div(10000).unwrap();
        let collateral_after_fee = collateral_out.checked_sub(fee).unwrap();

        require!(collateral_after_fee >= min_collateral_out, TradeError::SlippageExceeded);

        let (mint, source) = if sell_yes { (&self.yes_mint, &self.trader_yes) } else { (&self.no_mint, &self.trader_no) };
        burn(CpiContext::new(self.token_program.to_account_info(), Burn { mint: mint.to_account_info(), from: source.to_account_info(), authority: self.trader.to_account_info() }), amount)?;

        let config_key = self.config.key();
        let market_id_bytes = self.market.id.to_le_bytes();
        let market_seeds = &[crate::state::market::Market::SEED, config_key.as_ref(), &market_id_bytes, &[self.market.bump]];
        let market_signer = &[&market_seeds[..]];

        transfer_checked(CpiContext::new_with_signer(self.token_program.to_account_info(), TransferChecked { from: self.vault.to_account_info(), mint: self.collateral_mint.to_account_info(), to: self.trader_collateral.to_account_info(), authority: self.market.to_account_info() }, market_signer), collateral_after_fee, self.collateral_mint.decimals)?;

        self.market.reserves -= collateral_out;
        if sell_yes { self.market.yes_supply -= amount; } else { self.market.no_supply -= amount; }

        emit!(TokensSold { market_id: self.market.id, seller: self.trader.key(), is_yes: sell_yes, tokens_in: amount, collateral_out: collateral_after_fee });
        Ok(collateral_after_fee)
    }
}

// =============================================================================
// PUBLIC REDEMPTION (POST-RESO)
// =============================================================================

#[event]
pub struct PositionRedeemed {
    pub market_id: u64,
    pub redeemer: Pubkey,
    pub tokens_burned: u64,
    pub collateral_received: u64,
}

#[derive(Accounts)]
pub struct Redeem<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(seeds = [Config::SEED], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(mut, constraint = market.status == MarketStatus::Resolved @ RedeemError::NotResolved)]
    pub market: Account<'info, Market>,

    #[account(mut, constraint = yes_mint.key() == market.yes_mint)]
    pub yes_mint: InterfaceAccount<'info, Mint>,

    #[account(mut, constraint = no_mint.key() == market.no_mint)]
    pub no_mint: InterfaceAccount<'info, Mint>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,

    #[account(mut, associated_token::mint = yes_mint, associated_token::authority = user)]
    pub user_yes: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, associated_token::mint = no_mint, associated_token::authority = user)]
    pub user_no: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, associated_token::mint = collateral_mint, associated_token::authority = user)]
    pub user_collateral: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, associated_token::mint = collateral_mint, associated_token::authority = market)]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Redeem<'info> {
    pub fn redeem(&mut self) -> Result<u64> {
        let (user_balance, total_supply, winning_mint, user_account) = match self.market.outcome {
            Outcome::Yes => (self.user_yes.amount, self.market.yes_supply, &self.yes_mint, &self.user_yes),
            Outcome::No => (self.user_no.amount, self.market.no_supply, &self.no_mint, &self.user_no),
            Outcome::Undetermined => return err!(RedeemError::NotResolved),
        };

        require!(user_balance > 0, RedeemError::NoWinningTokens);

        let collateral_to_receive = (user_balance as u128).checked_mul(self.market.reserves as u128).unwrap().checked_div(total_supply as u128).unwrap() as u64;

        burn(CpiContext::new(self.token_program.to_account_info(), Burn { mint: winning_mint.to_account_info(), from: user_account.to_account_info(), authority: self.user.to_account_info() }), user_balance)?;

        let config_key = self.config.key();
        let market_id_bytes = self.market.id.to_le_bytes();
        let market_seeds = &[crate::state::market::Market::SEED, config_key.as_ref(), &market_id_bytes, &[self.market.bump]];
        let market_signer = &[&market_seeds[..]];

        transfer_checked(CpiContext::new_with_signer(self.token_program.to_account_info(), TransferChecked { from: self.vault.to_account_info(), mint: self.collateral_mint.to_account_info(), to: self.user_collateral.to_account_info(), authority: self.market.to_account_info() }, market_signer), collateral_to_receive, self.collateral_mint.decimals)?;

        self.market.reserves -= collateral_to_receive;
        emit!(PositionRedeemed { market_id: self.market.id, redeemer: self.user.key(), tokens_burned: user_balance, collateral_received: collateral_to_receive });
        Ok(collateral_to_receive)
    }
}

// =============================================================================
// ERRORS
// =============================================================================

#[error_code]
pub enum TradeError {
    #[msg("Market is not active")]
    MarketNotActive,
    #[msg("Market has ended")]
    MarketEnded,
    #[msg("Protocol is paused")]
    ProtocolPaused,
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
}

#[error_code]
pub enum RedeemError {
    #[msg("Market is not resolved")]
    NotResolved,
    #[msg("No winning tokens to redeem")]
    NoWinningTokens,
}
