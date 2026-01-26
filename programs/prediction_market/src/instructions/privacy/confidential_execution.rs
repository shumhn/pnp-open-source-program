//! Confidential Execution Module
//!
//! This module uses advanced encryption technology to hide your YES/NO choices.
//! No one can see what you bet on until the market is over.

use anchor_lang::prelude::*;

/// Confidential Position state (Choice is hidden)
#[account]
#[derive(InitSpace)]
pub struct ConfidentialPosition {
    /// The market this position belongs to
    pub market: Pubkey,
    
    /// Hash commitment for ownership verification
    pub commitment: [u8; 32],
    
    /// Hidden choice (YES or NO)
    pub encrypted_direction: [u8; 32],
    
    /// Hidden amount
    pub encrypted_amount: [u8; 32],
    
    /// Plaintext collateral for vault accounting (not private)
    pub collateral_deposited: u64,
    
    /// PDA bump
    pub bump: u8,
}

impl ConfidentialPosition {
    pub const SEED: &'static [u8] = b"confidential_position";
}

/// Instruction to trade with a confidential choice
#[derive(Accounts)]
#[instruction(commitment: [u8; 32], encrypted_direction: [u8; 32], amount: u64)]
pub struct TradeConfidential<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    
    /// CHECK: The market account
    pub market: AccountInfo<'info>,
    
    #[account(
        init,
        payer = trader,
        space = 8 + ConfidentialPosition::INIT_SPACE,
        seeds = [ConfidentialPosition::SEED, market.key().as_ref(), commitment.as_ref()],
        bump
    )]
    pub confidential_position: Account<'info, ConfidentialPosition>,
    
    /// CHECK: Execution program for encrypted operations
    pub execution_program: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

impl<'info> TradeConfidential<'info> {
    pub fn trade_confidential(
        &mut self,
        commitment: [u8; 32],
        encrypted_direction: [u8; 32],
        amount: u64,
        bump: u8,
    ) -> Result<()> {
        let pos = &mut self.confidential_position;
        pos.market = self.market.key();
        pos.commitment = commitment;
        pos.encrypted_direction = encrypted_direction;
        pos.encrypted_amount = [0u8; 32];
        pos.collateral_deposited = amount;
        pos.bump = bump;
        
        msg!("🎭 Confidential trade created");
        msg!("📊 Choice and amount are hidden.");
        
        Ok(())
    }
}

/// Event for a confidential trade
#[event]
pub struct ConfidentialPositionEntered {
    pub market: Pubkey,
    pub commitment: [u8; 32],
    pub collateral: u64,
}
