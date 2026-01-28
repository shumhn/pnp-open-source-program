# Technical Explanation: Price Privacy (MEV & Sandwiching)

## 1. The Problem: "The Sandwich Attack"
In public AMMs (like Uniswap or standard prediction markets), the token reserves are public variables.
- **The Bot**: Monitors the mempool for large buy orders.
- **The Calculation**: The bot calculates exactly how much your trade will move the price.
- **The Execution**: It buys before you (Front-run), you buy (Pump), and it sells after you (Back-run).
This "Sandwich" drains 1-5% of your trade value as pure profit for the bot. For institutions, this is an unacceptable **slippage tax**.

---

## 2. The Solution: Encrypted AMM State
We hide the "Real Odds" of the market. The AMM logic runs inside a secure, encrypted state that only reveal-hash owners can verify.

### Technical Reasoning
We separate the **Public Volume** from the **Private Reserves**.
1. Reserves ($X$ and $Y$) are stored as homomorphically encrypted ciphers.
2. The user submits a trade with a "Maximum Slippage" guard.
3. The program calculates if the trade is valid internally, updates the ciphers, but never reveals the new $X$ and $Y$ values to the public ledger.

---

## 3. High-Fidelity Pseudo-Code

### The Encrypted State Schema
```rust
struct DarkPoolReserves {
    encrypted_yes: Cipher64,
    encrypted_no: Cipher64,
    k_invariant: Cipher128, // The Pythagorean radius (R)
}
```

### The Shielded Trade (Program Logic)
The contract performs the pricing math in the "Dark."
```rust
pub fn execute_shielded_buy(ctx, volume: u64, slippage_guard: u64) {
    let reserves = &mut ctx.accounts.dark_pool;

    // 1. Internal Decryption (Instruction Context)
    // Decrypts the YES/NO balance only for the duration of this call
    let (real_yes, real_no) = decrypt_AMM_state(reserves);

    // 2. Calculate Outcome Tokens
    // tokens_out = f(volume, real_yes, real_no)
    let tokens_to_mint = bonding_curve::calculate_out(volume, real_yes, real_no);

    // 3. Verify Guard Rails
    let impact = calculate_impact(tokens_to_mint, volume);
    require!(impact <= slippage_guard, Error::SlippageExceeded);

    // 4. Update the "Black Box"
    // Re-encrypt the new balances and save back to PDA
    reserves.encrypted_yes = encrypt(real_yes + volume);
    reserves.encrypted_no = encrypt(real_no); // remains same for buy_yes
}
```

---

## 4. Why this is Secure
A bot looking at the blockchain sees that a buy happened, but it doesn't know the **Market Depth**. 
- Is the pool $1,000 deep or $1,000,000 deep?
- Are the current odds 50/50 or 90/10?
If the bot tries to "Sandwich" you, they are betting blind. Because they cannot calculate the price impact, the **Expected Value (EV)** of their attack drops to zero. This makes the Private PNP protocol the only "Institutional Safe" AMM on Solana.
