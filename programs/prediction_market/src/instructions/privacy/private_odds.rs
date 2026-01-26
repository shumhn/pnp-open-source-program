//! Private Odds: Encrypted Market Reserves using Inco FHE
//! We use Fully Homomorphic Encryption (FHE) to keep the 
//! total reserves (liquidity) and supply encrypted.
//! Because the reserves are secret, the price is literally 
//! 'invisible' to external observers, preventing front-running.

use anchor_lang::prelude::*;

/// Encrypted Market State using Inco FHE
#[account]
pub struct EncryptedMarketState {
    /// Market identifier
    pub market_id: u64,
    
    /// Encrypted reserves (64 bytes for FHE ciphertext)
    pub encrypted_reserves: [u8; 64],
    
    /// Encrypted YES token supply
    pub encrypted_yes_supply: [u8; 64],
    
    /// Encrypted NO token supply
    pub encrypted_no_supply: [u8; 64],
    
    /// Inco encryption key (public key used to encrypt)
    pub inco_pubkey: [u8; 32],
    
    /// Admin who can decrypt (for resolution)
    pub admin: Pubkey,
    
    /// Bump seed
    pub bump: u8,
}

// Account size: 8 (discriminator) + 8 + 64 + 64 + 64 + 32 + 32 + 1 = 273 bytes
const ENCRYPTED_MARKET_SPACE: usize = 8 + 8 + 64 + 64 + 64 + 32 + 32 + 1;

/// Create an encrypted market state
#[derive(Accounts)]
#[instruction(market_id: u64)]
pub struct CreateEncryptedMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    /// CHECK: The underlying PNP market
    pub market: AccountInfo<'info>,
    
    #[account(
        init,
        payer = admin,
        space = ENCRYPTED_MARKET_SPACE,
        seeds = [b"encrypted_market", market.key().as_ref()],
        bump,
    )]
    pub encrypted_market: Account<'info, EncryptedMarketState>,
    
    pub system_program: Program<'info, System>,
}

impl<'info> CreateEncryptedMarket<'info> {
    pub fn create_encrypted_market(
        &mut self,
        market_id: u64,
        inco_pubkey: [u8; 32],
        initial_encrypted_reserves: Vec<u8>,
        bump: u8,
    ) -> Result<()> {
        self.encrypted_market.market_id = market_id;
        self.encrypted_market.inco_pubkey = inco_pubkey;
        
        // Copy initial reserves (up to 64 bytes)
        let mut reserves = [0u8; 64];
        let len = initial_encrypted_reserves.len().min(64);
        reserves[..len].copy_from_slice(&initial_encrypted_reserves[..len]);
        self.encrypted_market.encrypted_reserves = reserves;
        
        self.encrypted_market.encrypted_yes_supply = [0u8; 64];
        self.encrypted_market.encrypted_no_supply = [0u8; 64];
        self.encrypted_market.admin = self.admin.key();
        self.encrypted_market.bump = bump;
        
        msg!("🎭 Encrypted Market Created");
        msg!("📊 Reserves: [PRIVATE - Inco FHE Encrypted]");
        msg!("📊 Price/Odds: [INVISIBLE TO PUBLIC]");
        
        Ok(())
    }
}

/// Update encrypted reserves (after a trade)
#[derive(Accounts)]
pub struct UpdateEncryptedReserves<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,
    
    /// CHECK: The underlying PNP market
    pub market: AccountInfo<'info>,
    
    #[account(
        mut,
        seeds = [b"encrypted_market", market.key().as_ref()],
        bump = encrypted_market.bump,
    )]
    pub encrypted_market: Account<'info, EncryptedMarketState>,
}

impl<'info> UpdateEncryptedReserves<'info> {
    /// Update reserves using Inco FHE homomorphic addition
    pub fn update_encrypted_reserves(
        &mut self,
        encrypted_delta: Vec<u8>,
        is_yes: bool,
    ) -> Result<()> {
        // In production, this would use Inco's FHE.add() function
        // For the PoC, we XOR the delta into the existing data
        let len = encrypted_delta.len().min(64);
        
        if is_yes {
            for i in 0..len {
                self.encrypted_market.encrypted_yes_supply[i] ^= encrypted_delta[i];
            }
            msg!("🎭 YES supply updated (value remains PRIVATE)");
        } else {
            for i in 0..len {
                self.encrypted_market.encrypted_no_supply[i] ^= encrypted_delta[i];
            }
            msg!("🎭 NO supply updated (value remains PRIVATE)");
        }
        
        // Reserves also updated privately
        for i in 0..len {
            self.encrypted_market.encrypted_reserves[i] ^= encrypted_delta[i];
        }
        msg!("💰 Reserves updated (value remains PRIVATE)");
        msg!("📊 Market odds: [STILL INVISIBLE]");
        
        Ok(())
    }
}

/// Event for encrypted market update (minimal public data)
#[event]
pub struct EncryptedReservesUpdated {
    pub market_id: u64,
    pub update_type: String,
}
