//! Market Resolution
//!
//! This module handles the resolution of prediction markets by authorized oracles.
//! 
//! ## Resolution Flow
//!
//! 1. Market end time passes
//! 2. Oracle/AI analyzes the outcome
//! 3. Oracle calls `resolve_market` with the result
//! 4. Market transitions to Resolved status
//! 5. Winners can redeem their tokens
//!
//! ## Oracle Integration
//!
//! The oracle can be:
//! - **AI Agent**: An autonomous agent that monitors real-world events
//! - **Multisig**: A committee of trusted resolvers
//! - **Decentralized Oracle**: Integration with Pyth, Chainlink, etc.
//! - **UMA-style Optimistic Oracle**: Dispute-based resolution

use anchor_lang::prelude::*;

use crate::state::{Config, Market, MarketStatus, Outcome};

/// Event emitted when a market is resolved
#[event]
pub struct MarketResolved {
    pub market_id: u64,
    pub outcome: Outcome,
    pub resolver: Pubkey,
    pub timestamp: i64,
}

/// Accounts for market resolution
#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    /// Oracle authorized to resolve markets
    #[account(
        constraint = oracle.key() == config.oracle @ ResolveError::Unauthorized
    )]
    pub oracle: Signer<'info>,

    /// Protocol configuration
    #[account(
        seeds = [Config::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    /// Market to resolve
    #[account(
        mut,
        constraint = market.status == MarketStatus::Active || 
                     market.status == MarketStatus::Ended @ ResolveError::CannotResolve,
    )]
    pub market: Account<'info, Market>,
}

impl<'info> ResolveMarket<'info> {
    /// Resolve the market with the winning outcome
    pub fn resolve_market(&mut self, yes_wins: bool) -> Result<()> {
        let clock = Clock::get()?;
        
        // Ensure market has ended
        require!(
            clock.unix_timestamp >= self.market.end_time as i64,
            ResolveError::MarketNotEnded
        );

        // Set the outcome
        self.market.outcome = if yes_wins {
            Outcome::Yes
        } else {
            Outcome::No
        };
        self.market.status = MarketStatus::Resolved;

        emit!(MarketResolved {
            market_id: self.market.id,
            outcome: self.market.outcome,
            resolver: self.oracle.key(),
            timestamp: clock.unix_timestamp,
        });

        msg!(
            "Market {} resolved: {:?}",
            self.market.id,
            self.market.outcome
        );

        Ok(())
    }
}

#[error_code]
pub enum ResolveError {
    #[msg("Only authorized oracle can resolve markets")]
    Unauthorized,
    #[msg("Market cannot be resolved in current state")]
    CannotResolve,
    #[msg("Market has not ended yet")]
    MarketNotEnded,
}
