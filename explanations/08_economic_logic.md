# Technical Explanation: Economic Logic (AMM Invariant)

## 1. The Problem: "Price Distortion in Prediction Markets"
Standard Automated Market Makers (like Uniswap's $xy=k$) are suboptimal for prediction markets. 
- **The Issue**: In a prediction market, tokens represent a probability from 0.0 to 1.0. 
- **The Distortion**: In $xy=k$, prices can act erratically when reserves are low. 
- **The Conflict**: Probability doesn't follow a hyperbola; it follows a normalized distribution where the sum of outcomes squared equals the total potential.

---

## 2. The Solution: Pythagorean AMM ($R = \sqrt{X^2 + Y^2}$)
We use a **Hybrid Pythagorean Invariant**. This is the state-of-the-art model for decentralized probability markets.

### Technical Reasoning
The model ensures that token pricing is a direct reflection of "Probability Confidence."
1. **The Invariant ($R$)**: The radius of the curve represents the total liquidity (locked collateral) in the market.
2. **Probability Scaling**: The price of a outcome is defined by its supply relative to the total radius. 
3. **Squared Normalization**: Prices follow $P_{yes}^2 + P_{no}^2 = 1$. This maintains a stable "Probability Surface" even as whales enter large positions.

---

## 3. High-Fidelity Pseudo-Code

### The Invariant Logic (Core AMM)
```rust
// Math Invariant Library
pub fn calculate_R(x: u64, y: u64) -> u64 {
    // R = SQRT(x^2 + y^2)
    let squared_sum = x.checked_pow(2).unwrap() + y.checked_pow(2).unwrap();
    return squared_sum.sqrt();
}

// Pricing Formula
pub fn get_price(supply: u64, r: u64) -> f64 {
    // Price = Supply / R
    return (supply as f64) / (r as f64);
}
```

### Trading Math (Buy Instruction)
```rust
pub fn buy_outcome(ctx, deposit_amount: u64, is_yes: bool) {
    let market = &mut ctx.accounts.market;
    
    // 1. Increase Reserves
    let old_r = market.reserves;
    let new_r = old_r + deposit_amount;

    // 2. Maintaining the Curve
    // If buying YES, we keep the NO supply fixed and grow the YES supply 
    // to match the new Radius (R).
    if is_yes {
        // new_yes = SQRT(new_r^2 - no_supply^2)
        let new_yes = sqrt(new_r.pow(2) - market.no_supply.pow(2));
        let tokens_to_mint = new_yes - market.yes_supply;
        
        market.yes_supply = new_yes;
        market.reserves = new_r;
        
        mint_tokens_to_user(ctx.accounts.user, tokens_to_mint);
    }
}
```

---

## 4. Why this is Secure & Efficient
- **Zero Probability Leak**: The prices are naturally bounded between 0 and 1. You can never have a token worth more than the total pool (unlike $xy=k$ which has theoretical infinity).
- **Institutional Depth**: Whales can model their "Price Impact" with high precision. Because the curve is a circle, the slippage is linear and predictable, which professional traders require for high-stakes positions. 
- **Economic Fairness**: Early liquidity providers and market creators are rewarded with stable odds that represent true event probability rather than artificial AMM mechanics.
