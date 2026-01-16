//! Permissionless Market Creation
//!
//! Anyone can create a prediction market by:
//! 1. Defining a yes/no question
//! 2. Setting an end time
//! 3. Providing initial liquidity
//!
//! The creator receives initial YES and NO tokens proportional to their liquidity.

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, MintTo, TokenAccount, TokenInterface, TransferChecked, mint_to, transfer_checked},
};

use crate::amm::PythagoreanCurve;
use crate::state::{Config, Market, MarketStatus, Outcome};

/// Event emitted when a new market is created
#[event]
pub struct MarketCreated {
    pub market_id: u64,
    pub creator: Pubkey,
    pub question: String,
    pub end_time: u64,
    pub initial_liquidity: u64,
}

/// Accounts for creating a new prediction market
#[derive(Accounts)]
pub struct CreateMarket<'info> {
    /// Market creator (pays for accounts, provides liquidity)
    #[account(mut)]
    pub creator: Signer<'info>,

    /// Global protocol configuration
    #[account(
        mut,
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    /// The new market account
    #[account(
        init,
        payer = creator,
        space = 8 + Market::INIT_SPACE,
        seeds = [Market::SEED, config.market_count.to_le_bytes().as_ref()],
        bump,
    )]
    pub market: Account<'info, Market>,

    /// YES token mint (created for this market)
    #[account(
        init,
        payer = creator,
        mint::decimals = collateral_mint.decimals,
        mint::authority = config,
        seeds = [b"yes_mint", config.market_count.to_le_bytes().as_ref()],
        bump,
    )]
    pub yes_mint: InterfaceAccount<'info, Mint>,

    /// NO token mint (created for this market)
    #[account(
        init,
        payer = creator,
        mint::decimals = collateral_mint.decimals,
        mint::authority = config,
        seeds = [b"no_mint", config.market_count.to_le_bytes().as_ref()],
        bump,
    )]
    pub no_mint: InterfaceAccount<'info, Mint>,

    /// Collateral token mint
    #[account(
        constraint = collateral_mint.key() == config.collateral_mint
    )]
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    /// Creator's collateral token account
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = creator,
    )]
    pub creator_collateral: InterfaceAccount<'info, TokenAccount>,

    /// Market's reserve vault
    #[account(
        init,
        payer = creator,
        associated_token::mint = collateral_mint,
        associated_token::authority = market,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    /// Creator's YES token account
    #[account(
        init,
        payer = creator,
        associated_token::mint = yes_mint,
        associated_token::authority = creator,
    )]
    pub creator_yes: InterfaceAccount<'info, TokenAccount>,

    /// Creator's NO token account
    #[account(
        init,
        payer = creator,
        associated_token::mint = no_mint,
        associated_token::authority = creator,
    )]
    pub creator_no: InterfaceAccount<'info, TokenAccount>,

    /// Token program
    pub token_program: Interface<'info, TokenInterface>,
    /// Associated token program
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// System program
    pub system_program: Program<'info, System>,
}

impl<'info> CreateMarket<'info> {
    pub fn create_market(
        &mut self,
        question: String,
        end_time: u64,
        initial_liquidity: u64,
        bumps: CreateMarketBumps,
    ) -> Result<()> {
        let clock = Clock::get()?;
        
        // Validations
        require!(!self.config.paused, CreateMarketError::ProtocolPaused);
        require!(end_time > clock.unix_timestamp as u64, CreateMarketError::InvalidEndTime);
        require!(initial_liquidity >= self.config.min_liquidity, CreateMarketError::InsufficientLiquidity);
        require!(question.len() <= 256, CreateMarketError::QuestionTooLong);

        let market_id = self.config.market_count;

        // Calculate initial token amounts
        // For balanced market: YES = NO = R / √2
        let reserves = initial_liquidity;
        let token_amount = integer_sqrt((reserves as u128 * reserves as u128) / 2) as u64;

        // Transfer collateral to vault
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

        // Mint YES tokens to creator
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

        // Mint NO tokens to creator
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

        // Initialize market state
        self.market.set_inner(Market {
            id: market_id,
            creator: self.creator.key(),
            question: question.clone(),
            end_time,
            created_at: clock.unix_timestamp as u64,
            yes_mint: self.yes_mint.key(),
            no_mint: self.no_mint.key(),
            collateral_mint: self.collateral_mint.key(),
            reserves: initial_liquidity,
            yes_supply: token_amount,
            no_supply: token_amount,
            status: MarketStatus::Active,
            outcome: Outcome::Undetermined,
            bump: bumps.market,
        });

        // Increment market counter
        self.config.market_count += 1;

        emit!(MarketCreated {
            market_id,
            creator: self.creator.key(),
            question,
            end_time,
            initial_liquidity,
        });

        Ok(())
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
}
