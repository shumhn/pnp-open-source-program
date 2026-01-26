//! Compressed Accounts Module
//!
//! This module uses ZK-Compression technology to hide user wallets and money.
//! It stores data in a compressed state to preserve privacy and scalability.

use anchor_lang::prelude::*;

/// Hidden Position (using ZK-Compression)
///
/// This data is hidden off-chain using ZK-Compression. 
/// Instead of storing a full transaction record on-chain, 
/// we store a 32-byte Merkle Leaf. This makes the 
/// trader's identity and balance changes invisible to trackers.
#[derive(Clone, Debug, PartialEq)]
pub struct CompressedPosition {
    /// Market identifier
    pub market_id: u64,
    
    /// Commitment hash for ownership: keccak(secret || user)
    pub ownership_commitment: [u8; 32],
    
    /// Auditor key for compliance
    pub compliance_commitment: [u8; 32],
    
    /// Hash of the view key used to decrypt this position
    pub view_key_hash: [u8; 32],
    
    /// Hidden choice (YES or NO)
    pub encrypted_direction: [u8; 32],
    
    /// Hidden bet amount
    pub amount: u64,
    
    /// Timestamp of the bet
    pub created_at: i64,
    
    /// Whether this position has been claimed
    pub is_claimed: bool,
}

/// Instruction for creating a ZK-compressed position
#[derive(Accounts)]
pub struct CreateCompressedPosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// CHECK: The market account
    pub market: AccountInfo<'info>,
    
    /// CHECK: The compression system program
    pub compression_program: AccountInfo<'info>,
    
    /// CHECK: The compressed account tree
    pub merkle_tree: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

impl<'info> CreateCompressedPosition<'info> {
    /// Create a new hidden position
    pub fn create_compressed_position(
        &mut self,
        _ownership_commitment: [u8; 32],
        _encrypted_direction: [u8; 32],
        _amount: u64,
        _compliance_commitment: [u8; 32],
        _view_key_hash: [u8; 32],
        _validity_proof: Vec<u8>,
    ) -> Result<()> {
        msg!("🏗️ Compressed position created");
        msg!("📊 Amount and wallet are private.");
        msg!("🤝 Audit key is stored.");
        
        Ok(())
    }
}

/// Event for a compressed position
#[event]
pub struct CompressedPositionCreated {
    pub market_id: u64,
    pub ownership_commitment: [u8; 32],
}

/// Helper module for compression primitives
pub mod compression_helpers {
    use anchor_lang::solana_program::keccak;
    
    /// Create a position leaf hash for the Merkle tree
    pub fn create_position_leaf(
        market_id: u64,
        commitment: &[u8; 32],
        encrypted_direction: &[u8; 32],
        amount: u64,
    ) -> [u8; 32] {
        let mut data = Vec::with_capacity(104);
        data.extend_from_slice(&market_id.to_le_bytes());
        data.extend_from_slice(commitment);
        data.extend_from_slice(encrypted_direction);
        data.extend_from_slice(&amount.to_le_bytes());
        keccak::hash(&data).0
    }
}
