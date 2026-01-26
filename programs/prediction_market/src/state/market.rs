//! Prediction Market State
//!
//! Each market represents a single yes/no prediction with its own liquidity pool.

use anchor_lang::prelude::*;

/// Individual prediction market account
///
/// Seeds: ["market", market_id.to_le_bytes()]
#[account]
#[derive(InitSpace)]
pub struct Market {
    /// Unique market identifier
    pub id: u64,

    /// Market creator's address
    pub creator: Pubkey,

    /// The prediction question
    /// Example: "Will ETH flip BTC by market cap in 2025?"
    #[max_len(256)]
    pub question: String,

    /// Unix timestamp when trading ends
    pub end_time: u64,

    /// Unix timestamp when market was created
    pub created_at: u64,

    /// YES token mint address
    pub yes_mint: Pubkey,

    /// NO token mint address
    pub no_mint: Pubkey,

    /// Collateral token mint address
    pub collateral_mint: Pubkey,

    /// Total collateral reserves in the pool
    pub reserves: u64,

    /// Total YES tokens minted
    pub yes_supply: u64,

    /// Total NO tokens minted
    pub no_supply: u64,

    /// Shielded reserve commitment: hash(reserves, blinding_factor)
    /// Allows hiding exact amounts while proving validity
    pub shielded_reserve_commitment: [u8; 32],

    /// Blinding factor for reserve commitment (updated each trade)
    pub reserve_blinding: [u8; 32],

    /// Market resolution status
    pub status: MarketStatus,

    /// Winning outcome (only valid after resolution)
    pub outcome: Outcome,

    /// PDA bump seed
    pub bump: u8,
}

impl Market {
    pub const SEED: &'static [u8] = b"market";

    /// Create a commitment for the current reserve amount
    /// commitment = keccak256(reserves || blinding_factor)
    pub fn compute_reserve_commitment(reserves: u64, blinding: &[u8; 32]) -> [u8; 32] {
        use anchor_lang::solana_program::keccak;
        let mut data = [0u8; 40]; // 8 bytes for u64 + 32 bytes for blinding
        data[..8].copy_from_slice(&reserves.to_le_bytes());
        data[8..].copy_from_slice(blinding);
        keccak::hash(&data).0
    }

    /// Update the shielded commitment after a trade
    pub fn update_commitment(&mut self, new_blinding: [u8; 32]) {
        self.reserve_blinding = new_blinding;
        self.shielded_reserve_commitment = Self::compute_reserve_commitment(self.reserves, &new_blinding);
    }
}

/// A privacy claim representing a pending private payout.
/// 
/// This acts as the 'Secret Ballot Box' for redemptions.
/// The original trader commits to a hash. Anyone with the secret
/// can later claim to any wallet, breaking the on-chain link.
///
/// Seeds: ["privacy_claim", market.key().as_ref(), commitment.as_ref()]
#[account]
#[derive(InitSpace)]
pub struct PrivacyClaim {
    pub market: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub lock_until: i64, 
    pub commitment: [u8; 32],
    pub redeemed: bool,
    pub nonce: u64, // Anti-replay nonce
    pub bump: u8,
}

impl PrivacyClaim {
    pub const SEED: &'static [u8] = b"privacy_claim";
}

/// A privacy position representing ghost ownership of outcome tokens.
/// 
/// This prevents bots from seeing which wallet owns which position.
/// Seeds: ["privacy_position", market.key().as_ref(), commitment.as_ref()]
#[account]
#[derive(InitSpace)]
pub struct PrivacyPosition {
    pub market: Pubkey,
    pub commitment: [u8; 32],
    pub yes_amount: u64,
    pub no_amount: u64,
    pub bump: u8,
}

impl PrivacyPosition {
    pub const SEED: &'static [u8] = b"privacy_position";
}

/// A shielded position with encrypted direction for Blind Betting.
/// 
/// This is the most advanced privacy primitive: the blockchain cannot
/// see whether the user bet YES or NO until the market resolves and
/// the user reveals their secret.
///
/// Seeds: ["shielded_position", market.key().as_ref(), commitment.as_ref()]
#[account]
#[derive(InitSpace)]
pub struct ShieldedPosition {
    /// The market this position belongs to
    pub market: Pubkey,
    /// Hash commitment: keccak256(secret || trader)
    pub commitment: [u8; 32],
    /// Encrypted direction: encrypt(buy_yes, secret)
    /// Only the secret holder can decrypt this to prove their bet
    pub direction_cipher: [u8; 32],
    /// Unified shielded balance (hides YES vs NO split)
    pub shielded_amount: u64,
    /// Collateral deposited (for accurate payout calculation)
    pub collateral_deposited: u64,
    /// PDA bump seed
    pub bump: u8,
}

impl ShieldedPosition {
    pub const SEED: &'static [u8] = b"shielded_position";
    
    /// Encrypt direction using XOR with secret hash
    /// Simple but effective for hackathon demo
    pub fn encrypt_direction(buy_yes: bool, secret: &[u8; 32]) -> [u8; 32] {
        let mut cipher = [0u8; 32];
        cipher[0] = if buy_yes { 1 } else { 0 };
        // XOR with secret for obfuscation
        for i in 0..32 {
            cipher[i] ^= secret[i];
        }
        cipher
    }
    
    /// Decrypt direction using XOR with secret
    pub fn decrypt_direction(cipher: &[u8; 32], secret: &[u8; 32]) -> bool {
        let mut decrypted = cipher[0];
        decrypted ^= secret[0];
        decrypted == 1
    }
}


/// Market lifecycle status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug, Default)]
pub enum MarketStatus {
    /// Market is open for trading
    #[default]
    Active,
    /// Trading ended, awaiting resolution
    Ended,
    /// Market has been resolved
    Resolved,
    /// Market was cancelled/voided
    Cancelled,
}

/// Prediction outcome
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Debug, Default)]
pub enum Outcome {
    /// Not yet determined
    #[default]
    Undetermined,
    /// YES outcome occurred
    Yes,
    /// NO outcome occurred
    No,
}
