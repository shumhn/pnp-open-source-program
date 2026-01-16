//! # Pythagorean Bonding Curve
//!
//! This module implements the **Pythagorean AMM** for prediction markets.
//!
//! ## The Core Invariant
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                                                              │
//! │              R = √(YES² + NO²)                              │
//! │                                                              │
//! │   Where:                                                     │
//! │   • R = Total collateral reserves                           │
//! │   • YES = Supply of YES outcome tokens                      │
//! │   • NO = Supply of NO outcome tokens                        │
//! │                                                              │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Why Pythagorean?
//!
//! This formula creates a beautiful property for prediction markets:
//!
//! **Prices naturally represent probabilities!**
//!
//! ```text
//! YES_price = YES / R = YES / √(YES² + NO²)
//! NO_price  = NO / R  = NO / √(YES² + NO²)
//!
//! And critically: YES_price² + NO_price² = 1
//! ```
//!
//! This means:
//! - At 50/50 odds (YES = NO), each price ≈ 0.707 (1/√2)
//! - As YES increases relative to NO, YES_price approaches 1
//! - Prices always remain bounded between 0 and 1
//!
//! ## Token Minting Formula
//!
//! When a user deposits `L` collateral to buy YES tokens:
//!
//! ```text
//! 1. new_R = R + L              (reserves increase)
//! 2. new_YES = √(new_R² - NO²)  (solve invariant for YES)
//! 3. tokens_out = new_YES - YES (mint the difference)
//! ```
//!
//! ## Token Burning Formula
//!
//! When a user burns YES tokens to withdraw collateral:
//!
//! ```text
//! 1. new_YES = YES - tokens_burned
//! 2. new_R = √(new_YES² + NO²)  (solve invariant for R)
//! 3. collateral_out = R - new_R (release the difference)
//! ```

use anchor_lang::prelude::*;

/// Errors specific to the Pythagorean bonding curve
#[error_code]
pub enum AmmError {
    #[msg("Invalid reserves: must be positive")]
    InvalidReserves,
    #[msg("Invalid token supplies")]
    InvalidSupplies,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
    #[msg("Cannot burn more tokens than supply")]
    InsufficientTokens,
    #[msg("No tokens to mint")]
    NoTokensToMint,
}

/// Precision scale factor to prevent overflow while maintaining accuracy
/// Divide inputs by this, compute, then multiply result back
const PRECISION_SCALE: u128 = 1_000;

/// Pythagorean Bonding Curve for Prediction Markets
///
/// Implements R = √(YES² + NO²) invariant
pub struct PythagoreanCurve;

impl PythagoreanCurve {
    /// Calculate tokens to mint when adding collateral (buying tokens)
    ///
    /// Formula: new_YES = √(new_R² - NO²), tokens = new_YES - old_YES
    ///
    /// # Arguments
    /// * `reserves` - Current total reserves (R)
    /// * `target_supply` - Supply of token being bought (A: YES or NO)
    /// * `other_supply` - Supply of the other token (B)
    /// * `collateral_in` - Amount of collateral being deposited (L)
    ///
    /// # Returns
    /// * Amount of tokens to mint to the buyer
    ///
    /// # Example
    /// ```ignore
    /// // Market has 1000 reserves, 707 YES, 707 NO (balanced)
    /// // User wants to buy YES with 100 collateral
    /// let tokens = PythagoreanCurve::get_tokens_to_mint(1000, 707, 707, 100)?;
    /// // tokens ≈ 86 YES tokens
    /// ```
    pub fn get_tokens_to_mint(
        reserves: u64,
        target_supply: u64,
        other_supply: u64,
        collateral_in: u64,
    ) -> Result<u64> {
        // Input validation
        require!(reserves > 0, AmmError::InvalidReserves);
        require!(collateral_in > 0, AmmError::InvalidReserves);

        // Scale down to prevent overflow (maintains 3 decimal precision)
        let r = (reserves as u128) / PRECISION_SCALE;
        let a = (target_supply as u128) / PRECISION_SCALE;
        let b = (other_supply as u128) / PRECISION_SCALE;
        let l = (collateral_in as u128) / PRECISION_SCALE;

        // Step 1: new_R = R + L
        let new_r = r.checked_add(l).ok_or(AmmError::Overflow)?;

        // Step 2: new_R² and B²
        let new_r_squared = new_r.checked_mul(new_r).ok_or(AmmError::Overflow)?;
        let b_squared = b.checked_mul(b).ok_or(AmmError::Overflow)?;

        // Sanity check: new_R² must be >= B² for valid state
        require!(new_r_squared >= b_squared, AmmError::InvalidSupplies);

        // Step 3: new_A² = new_R² - B²
        let new_a_squared = new_r_squared
            .checked_sub(b_squared)
            .ok_or(AmmError::Overflow)?;

        // Step 4: new_A = √(new_A²)
        let new_a = sqrt(new_a_squared);

        // Step 5: tokens_out = new_A - old_A
        require!(new_a > a, AmmError::NoTokensToMint);
        let tokens_out = new_a.checked_sub(a).ok_or(AmmError::Overflow)?;

        // Scale back up
        let scaled_result = tokens_out
            .checked_mul(PRECISION_SCALE)
            .ok_or(AmmError::Overflow)?;

        Ok(scaled_result as u64)
    }

    /// Calculate collateral to release when burning tokens (selling)
    ///
    /// Formula: new_R = √(new_A² + B²), collateral = old_R - new_R
    ///
    /// # Arguments
    /// * `reserves` - Current total reserves (R)
    /// * `target_supply` - Supply of token being sold (A)
    /// * `other_supply` - Supply of the other token (B)
    /// * `tokens_to_burn` - Amount of tokens being burned
    ///
    /// # Returns
    /// * Amount of collateral to return to the seller
    pub fn get_reserve_to_release(
        reserves: u64,
        target_supply: u64,
        other_supply: u64,
        tokens_to_burn: u64,
    ) -> Result<u64> {
        require!(tokens_to_burn > 0, AmmError::InvalidReserves);
        require!(tokens_to_burn <= target_supply, AmmError::InsufficientTokens);

        // Scale down
        let r = (reserves as u128) / PRECISION_SCALE;
        let a = (target_supply as u128) / PRECISION_SCALE;
        let b = (other_supply as u128) / PRECISION_SCALE;
        let burn = (tokens_to_burn as u128) / PRECISION_SCALE;

        // Step 1: new_A = A - tokens_burned
        let new_a = a.checked_sub(burn).ok_or(AmmError::Overflow)?;

        // Step 2: new_A² and B²
        let new_a_squared = new_a.checked_mul(new_a).ok_or(AmmError::Overflow)?;
        let b_squared = b.checked_mul(b).ok_or(AmmError::Overflow)?;

        // Step 3: new_R² = new_A² + B²
        let new_r_squared = new_a_squared
            .checked_add(b_squared)
            .ok_or(AmmError::Overflow)?;

        // Step 4: new_R = √(new_R²)
        let new_r = sqrt(new_r_squared);

        // Step 5: collateral_out = R - new_R
        let collateral_out = r.saturating_sub(new_r);

        // Scale back up
        let scaled_result = collateral_out
            .checked_mul(PRECISION_SCALE)
            .ok_or(AmmError::Overflow)?;

        Ok(scaled_result as u64)
    }

    /// Get the current price of a token
    ///
    /// Price = A / R where R = √(A² + B²)
    ///
    /// This represents:
    /// - The marginal cost to buy the next infinitesimal token
    /// - A probability-like value between 0 and 1
    ///
    /// # Returns
    /// * Price in basis points (10000 = 1.0)
    ///
    /// # Example
    /// ```ignore
    /// // At 50/50 odds (YES = NO = 707, R = 1000)
    /// let price = PythagoreanCurve::get_price(1000, 707, 707)?;
    /// // price ≈ 7070 bps (0.707 or ~70.7%)
    /// // Note: In Pythagorean, balanced price is 1/√2 ≈ 0.707
    /// ```
    pub fn get_price(reserves: u64, target_supply: u64, _other_supply: u64) -> Result<u64> {
        if reserves == 0 {
            return Ok(5000); // Default 50% if no liquidity
        }

        // Scale down
        let r = (reserves as u128) / PRECISION_SCALE;
        let a = (target_supply as u128) / PRECISION_SCALE;

        if r == 0 {
            return Ok(5000);
        }

        // Price = A / R (scaled to basis points)
        // price_bps = (A * 10000) / R
        let price_bps = a
            .checked_mul(10000)
            .ok_or(AmmError::Overflow)?
            .checked_div(r)
            .ok_or(AmmError::DivisionByZero)?;

        Ok(price_bps as u64)
    }

    /// Get prices for both YES and NO tokens
    ///
    /// # Returns
    /// * (yes_price_bps, no_price_bps) - both in basis points
    pub fn get_prices(
        reserves: u64,
        yes_supply: u64,
        no_supply: u64,
    ) -> Result<(u64, u64)> {
        let yes_price = Self::get_price(reserves, yes_supply, no_supply)?;
        let no_price = Self::get_price(reserves, no_supply, yes_supply)?;
        Ok((yes_price, no_price))
    }
}

/// Integer square root using Newton's method
///
/// Computes floor(√x) efficiently for any non-negative integer
///
/// # Algorithm
/// Uses iterative refinement: z = (x/z + z) / 2
/// Converges quadratically to √x
pub fn sqrt(x: u128) -> u128 {
    if x == 0 {
        return 0;
    }

    // Initial guess
    let mut z = (x + 1) / 2;
    let mut y = x;

    // Newton's method iteration
    while z < y {
        y = z;
        z = (x / z + z) / 2;
    }

    y
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt() {
        assert_eq!(sqrt(0), 0);
        assert_eq!(sqrt(1), 1);
        assert_eq!(sqrt(4), 2);
        assert_eq!(sqrt(9), 3);
        assert_eq!(sqrt(10), 3); // floor(√10) = 3
        assert_eq!(sqrt(100), 10);
        assert_eq!(sqrt(1000000), 1000);
    }

    #[test]
    fn test_invariant_holds() {
        // R = 1000, YES = NO = 707 (approximately R/√2)
        // √(707² + 707²) = √(999698) ≈ 999.8 ≈ 1000 ✓
        let r_squared = 707u128 * 707 + 707 * 707;
        let r = sqrt(r_squared);
        assert!(r >= 999 && r <= 1001);
    }

    #[test]
    fn test_balanced_market_prices() {
        // When YES = NO, both prices should be equal
        let reserves = 1_000_000u64; // 1M with scaling
        let yes_supply = 707_000u64;
        let no_supply = 707_000u64;

        let yes_price = PythagoreanCurve::get_price(reserves, yes_supply, no_supply).unwrap();
        let no_price = PythagoreanCurve::get_price(reserves, no_supply, yes_supply).unwrap();

        // Both should be around 7070 bps (70.7% = 1/√2)
        assert_eq!(yes_price, no_price);
        assert!(yes_price >= 7000 && yes_price <= 7200);
    }

    #[test]
    fn test_buy_increases_supply() {
        let reserves = 1_000_000u64;
        let yes_supply = 707_000u64;
        let no_supply = 707_000u64;
        let collateral_in = 100_000u64;

        let tokens_out = PythagoreanCurve::get_tokens_to_mint(
            reserves,
            yes_supply,
            no_supply,
            collateral_in,
        ).unwrap();

        assert!(tokens_out > 0);
        // New YES supply should maintain invariant
    }

    #[test]
    fn test_sell_returns_collateral() {
        let reserves = 1_000_000u64;
        let yes_supply = 800_000u64;
        let no_supply = 600_000u64;
        let tokens_to_burn = 50_000u64;

        let collateral_out = PythagoreanCurve::get_reserve_to_release(
            reserves,
            yes_supply,
            no_supply,
            tokens_to_burn,
        ).unwrap();

        assert!(collateral_out > 0);
        assert!(collateral_out < tokens_to_burn); // Should get less collateral than tokens burned
    }
}
