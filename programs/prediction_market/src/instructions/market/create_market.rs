//! Permissionless Market Creation Pipeline
//!
//! This file contains the complete modular pipeline for creating a market.
//! Due to Solana's 4KB stack limit, the creation process is split into
//! 4 atomic steps that must be called in sequence.
//!
//! Step 1: CreateMarketState - Initializes the market account.
//! Step 2: CreateMarketMints - Creates the YES and NO token mints.
//! Step 3: CreateMarketVaults - Creates the market's collateral vault and creator accounts.
//! Step 4: FundMarket - Transfers initial liquidity and mints initial tokens.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, MintTo, TokenAccount, TokenInterface, TransferChecked, mint_to, transfer_checked},
};

use crate::state::{Config, Market, MarketStatus, Outcome};

// =============================================================================
// STEP 1: CREATE MARKET STATE
// =============================================================================

/// Event emitted when market state is created
#[event]
pub struct MarketStateCreated {
    pub market_id: u64,
    pub creator: Pubkey,
    pub end_time: u64,
}

#[derive(Accounts)]
#[instruction(question: String, end_time: u64)]
pub struct CreateMarketState<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = creator,
        space = 8 + Market::INIT_SPACE,
        seeds = [Market::SEED, config.key().as_ref(), config.market_count.to_le_bytes().as_ref()],
        bump,
    )]
    pub market: Account<'info, Market>,

    /// CHECK: Validated in Step 4 (FundMarket)
    pub collateral_mint: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateMarketState<'info> {
    pub fn create_market_state(
        &mut self,
        question: String,
        end_time: u64,
        bumps: &CreateMarketStateBumps,
    ) -> Result<()> {
        let clock = Clock::get()?;
        
        require!(!self.config.paused, CreateMarketError::ProtocolPaused);
        require!(end_time > clock.unix_timestamp as u64, CreateMarketError::InvalidEndTime);
        require!(question.len() <= 256, CreateMarketError::QuestionTooLong);

        let market_id = self.config.market_count;

        self.market.set_inner(Market {
            id: market_id,
            creator: self.creator.key(),
            question,
            end_time,
            created_at: clock.unix_timestamp as u64,
            yes_mint: Pubkey::default(),
            no_mint: Pubkey::default(),
            collateral_mint: self.collateral_mint.key(),
            reserves: 0,
            yes_supply: 0,
            no_supply: 0,
            shielded_reserve_commitment: [0u8; 32],
            reserve_blinding: [0u8; 32],
            status: MarketStatus::Active,
            outcome: Outcome::Undetermined,
            bump: bumps.market,
        });

        self.config.market_count += 1;

        emit!(MarketStateCreated {
            market_id,
            creator: self.creator.key(),
            end_time,
        });

        Ok(())
    }
}

// =============================================================================
// STEP 2: CREATE MARKET MINTS
// =============================================================================

/// Event emitted when market mints are created
#[event]
pub struct MarketMintsCreated {
    pub market_id: u64,
    pub yes_mint: Pubkey,
    pub no_mint: Pubkey,
}

#[derive(Accounts)]
pub struct CreateMarketMints<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        constraint = market.creator == creator.key(),
        constraint = market.yes_mint == Pubkey::default(),
    )]
    pub market: Account<'info, Market>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = creator,
        mint::decimals = collateral_mint.decimals,
        mint::authority = config,
        seeds = [b"yes_mint", market.key().as_ref()],
        bump,
    )]
    pub yes_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = creator,
        mint::decimals = collateral_mint.decimals,
        mint::authority = config,
        seeds = [b"no_mint", market.key().as_ref()],
        bump,
    )]
    pub no_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateMarketMints<'info> {
    pub fn create_market_mints(&mut self) -> Result<()> {
        self.market.yes_mint = self.yes_mint.key();
        self.market.no_mint = self.no_mint.key();

        emit!(MarketMintsCreated {
            market_id: self.market.id,
            yes_mint: self.yes_mint.key(),
            no_mint: self.no_mint.key(),
        });

        Ok(())
    }
}

// =============================================================================
// STEP 3: CREATE MARKET VAULTS
// =============================================================================

/// Event emitted when market vaults are created
#[event]
pub struct MarketVaultsCreated {
    pub market_id: u64,
}

#[derive(Accounts)]
pub struct CreateMarketVaults<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        constraint = market.creator == creator.key(),
        constraint = market.reserves == 0,
        constraint = market.yes_mint != Pubkey::default(),
    )]
    pub market: Box<Account<'info, Market>>,

    /// CHECK: Manual validation to save stack
    #[account(constraint = yes_mint.key() == market.yes_mint)]
    pub yes_mint: AccountInfo<'info>,

    /// CHECK: Manual validation to save stack
    #[account(constraint = no_mint.key() == market.no_mint)]
    pub no_mint: AccountInfo<'info>,

    /// CHECK: Manual validation to save stack
    #[account(constraint = collateral_mint.key() == market.collateral_mint)]
    pub collateral_mint: AccountInfo<'info>,

    #[account(
        init,
        payer = creator,
        associated_token::mint = collateral_mint,
        associated_token::authority = market,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = creator,
        associated_token::mint = yes_mint,
        associated_token::authority = creator,
    )]
    pub creator_yes: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = creator,
        associated_token::mint = no_mint,
        associated_token::authority = creator,
    )]
    pub creator_no: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateMarketVaults<'info> {
    pub fn create_market_vaults(&mut self) -> Result<()> {
        emit!(MarketVaultsCreated {
            market_id: self.market.id,
        });
        Ok(())
    }
}

// =============================================================================
// STEP 4: FUND MARKET
// =============================================================================

/// Event emitted when market is funded
#[event]
pub struct MarketFunded {
    pub market_id: u64,
    pub initial_liquidity: u64,
}

#[derive(Accounts)]
pub struct FundMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Box<Account<'info, Config>>,

    #[account(
        mut,
        constraint = market.creator == creator.key(),
        constraint = market.reserves == 0,
        constraint = market.yes_mint != Pubkey::default(),
    )]
    pub market: Box<Account<'info, Market>>,

    /// CHECK: Manual validation to save stack
    #[account(mut, constraint = yes_mint.key() == market.yes_mint)]
    pub yes_mint: AccountInfo<'info>,

    /// CHECK: Manual validation to save stack
    #[account(mut, constraint = no_mint.key() == market.no_mint)]
    pub no_mint: AccountInfo<'info>,

    /// CHECK: Manual validation to save stack
    #[account(constraint = collateral_mint.key() == market.collateral_mint)]
    pub collateral_mint: InterfaceAccount<'info, Mint>,


    #[account(mut)]
    pub creator_collateral: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub creator_yes: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub creator_no: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> FundMarket<'info> {
    pub fn fund_market(&mut self, initial_liquidity: u64) -> Result<()> {
        require!(
            initial_liquidity >= self.config.min_liquidity,
            CreateMarketError::InsufficientLiquidity
        );

        let reserves = initial_liquidity;
        let token_amount = integer_sqrt((reserves as u128 * reserves as u128) / 2) as u64;

        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.creator_collateral.to_account_info(),
                    mint: self.collateral_mint.to_account_info(),
                    to: self.vault.to_account_info(),
                    authority: self.creator.to_account_info(),
                },
            ),
            initial_liquidity,
            self.collateral_mint.decimals,
        )?;

        let config_seeds = &[Config::SEED, &[self.config.bump]];
        let signer_seeds = &[&config_seeds[..]];

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.yes_mint.to_account_info(),
                    to: self.creator_yes.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            token_amount,
        )?;

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.no_mint.to_account_info(),
                    to: self.creator_no.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            token_amount,
        )?;

        self.market.reserves = initial_liquidity;
        self.market.yes_supply = token_amount;
        self.market.no_supply = token_amount;

        emit!(MarketFunded {
            market_id: self.market.id,
            initial_liquidity,
        });

        Ok(())
    }
}

// =============================================================================
// LEGACY / HELPERS
// =============================================================================

#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    /// CHECK: Legacy placeholder - use Step 1-4 pipeline
    pub config: AccountInfo<'info>,
    
    /// CHECK: Legacy placeholder - use Step 1-4 pipeline
    pub market: AccountInfo<'info>,
}

impl<'info> CreateMarket<'info> {
    pub fn create_market(
        &mut self,
        _question: String,
        _end_time: u64,
        _initial_liquidity: u64,
        _bumps: &CreateMarketBumps,
    ) -> Result<()> {
        err!(CreateMarketError::Deprecated)
    }
}

fn integer_sqrt(x: u128) -> u128 {
    if x == 0 { return 0; }
    let mut z = (x + 1) / 2;
    let mut y = x;
    while z < y {
        y = z;
        z = (x / z + z) / 2;
    }
    y
}

#[error_code]
pub enum CreateMarketError {
    #[msg("Protocol is paused")]
    ProtocolPaused,
    #[msg("End time must be in the future")]
    InvalidEndTime,
    #[msg("Initial liquidity below minimum")]
    InsufficientLiquidity,
    #[msg("Question exceeds maximum length")]
    QuestionTooLong,
    #[msg("Legacy instruction deprecated, use Step 1-4 pipeline")]
    Deprecated,
}
