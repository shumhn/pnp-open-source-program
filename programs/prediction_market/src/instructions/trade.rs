//! Token Trading
//!
//! Handles buying and selling of YES/NO outcome tokens using
//! the AMM bonding curve for price discovery.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
        Burn, mint_to, transfer_checked, burn,
    },
};

use crate::amm::PythagoreanCurve;
use crate::state::{Config, Market, MarketStatus};

/// Event emitted when tokens are bought
#[event]
pub struct TokensBought {
    pub market_id: u64,
    pub buyer: Pubkey,
    pub is_yes: bool,
    pub collateral_in: u64,
    pub tokens_out: u64,
}

/// Event emitted when tokens are sold
#[event]
pub struct TokensSold {
    pub market_id: u64,
    pub seller: Pubkey,
    pub is_yes: bool,
    pub tokens_in: u64,
    pub collateral_out: u64,
}

/// Accounts for trading operations
#[derive(Accounts)]
pub struct Trade<'info> {
    /// Trader
    #[account(mut)]
    pub trader: Signer<'info>,

    /// Protocol configuration
    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    /// Market being traded on
    #[account(
        mut,
        constraint = market.status == MarketStatus::Active @ TradeError::MarketNotActive,
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

    /// Trader's collateral account
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = trader,
    )]
    pub trader_collateral: InterfaceAccount<'info, TokenAccount>,

    /// Trader's YES token account
    #[account(
        init_if_needed,
        payer = trader,
        associated_token::mint = yes_mint,
        associated_token::authority = trader,
    )]
    pub trader_yes: InterfaceAccount<'info, TokenAccount>,

    /// Trader's NO token account
    #[account(
        init_if_needed,
        payer = trader,
        associated_token::mint = no_mint,
        associated_token::authority = trader,
    )]
    pub trader_no: InterfaceAccount<'info, TokenAccount>,

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

impl<'info> Trade<'info> {
    /// Buy YES or NO tokens
    pub fn buy_tokens(
        &mut self,
        amount: u64,
        buy_yes: bool,
        min_tokens_out: u64,
    ) -> Result<u64> {
        let clock = Clock::get()?;
        
        // Check market is still open for trading
        require!(
            clock.unix_timestamp < self.market.end_time as i64,
            TradeError::MarketEnded
        );
        require!(!self.config.paused, TradeError::ProtocolPaused);

        // Calculate fee
        let fee = amount
            .checked_mul(self.config.protocol_fee_bps)
            .unwrap()
            .checked_div(10000)
            .unwrap();
        let amount_after_fee = amount.checked_sub(fee).unwrap();

        // Calculate tokens to mint using bonding curve
        let (target_supply, other_supply) = if buy_yes {
            (self.market.yes_supply, self.market.no_supply)
        } else {
            (self.market.no_supply, self.market.yes_supply)
        };

        let tokens_out = PythagoreanCurve::get_tokens_to_mint(
            self.market.reserves,
            target_supply,
            other_supply,
            amount_after_fee,
        )?;

        // Slippage check
        require!(tokens_out >= min_tokens_out, TradeError::SlippageExceeded);

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
            amount_after_fee,
            self.collateral_mint.decimals,
        )?;

        // Mint tokens to trader
        let config_seeds = &[Config::SEED, &[self.config.bump]];
        let signer_seeds = &[&config_seeds[..]];

        let (mint, destination) = if buy_yes {
            (&self.yes_mint, &self.trader_yes)
        } else {
            (&self.no_mint, &self.trader_no)
        };

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: mint.to_account_info(),
                    to: destination.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            tokens_out,
        )?;

        // Update market state
        self.market.reserves = self.market.reserves.checked_add(amount_after_fee).unwrap();
        if buy_yes {
            self.market.yes_supply = self.market.yes_supply.checked_add(tokens_out).unwrap();
        } else {
            self.market.no_supply = self.market.no_supply.checked_add(tokens_out).unwrap();
        }

        emit!(TokensBought {
            market_id: self.market.id,
            buyer: self.trader.key(),
            is_yes: buy_yes,
            collateral_in: amount,
            tokens_out,
        });

        Ok(tokens_out)
    }

    /// Sell YES or NO tokens
    pub fn sell_tokens(
        &mut self,
        amount: u64,
        sell_yes: bool,
        min_collateral_out: u64,
    ) -> Result<u64> {
        let clock = Clock::get()?;
        
        require!(
            clock.unix_timestamp < self.market.end_time as i64,
            TradeError::MarketEnded
        );
        require!(!self.config.paused, TradeError::ProtocolPaused);

        // Calculate collateral to return
        let (target_supply, other_supply) = if sell_yes {
            (self.market.yes_supply, self.market.no_supply)
        } else {
            (self.market.no_supply, self.market.yes_supply)
        };

        let collateral_out = PythagoreanCurve::get_reserve_to_release(
            self.market.reserves,
            target_supply,
            other_supply,
            amount,
        )?;

        // Apply fee
        let fee = collateral_out
            .checked_mul(self.config.protocol_fee_bps)
            .unwrap()
            .checked_div(10000)
            .unwrap();
        let collateral_after_fee = collateral_out.checked_sub(fee).unwrap();

        // Slippage check
        require!(collateral_after_fee >= min_collateral_out, TradeError::SlippageExceeded);

        // Burn tokens
        let (mint, source) = if sell_yes {
            (&self.yes_mint, &self.trader_yes)
        } else {
            (&self.no_mint, &self.trader_no)
        };

        burn(
            CpiContext::new(
                self.token_program.to_account_info(),
                Burn {
                    mint: mint.to_account_info(),
                    from: source.to_account_info(),
                    authority: self.trader.to_account_info(),
                },
            ),
            amount,
        )?;

        // Transfer collateral back to trader
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
                    to: self.trader_collateral.to_account_info(),
                    authority: self.market.to_account_info(),
                },
                market_signer,
            ),
            collateral_after_fee,
            self.collateral_mint.decimals,
        )?;

        // Update market state
        self.market.reserves = self.market.reserves.checked_sub(collateral_out).unwrap();
        if sell_yes {
            self.market.yes_supply = self.market.yes_supply.checked_sub(amount).unwrap();
        } else {
            self.market.no_supply = self.market.no_supply.checked_sub(amount).unwrap();
        }

        emit!(TokensSold {
            market_id: self.market.id,
            seller: self.trader.key(),
            is_yes: sell_yes,
            tokens_in: amount,
            collateral_out: collateral_after_fee,
        });

        Ok(collateral_after_fee)
    }
}

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
