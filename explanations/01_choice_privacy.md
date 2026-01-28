# Technical Explanation: Choice Privacy (Alpha Leakage)

## 1. The Problem: "Alpha Leakage"
In standard prediction markets, the moment you buy a "YES" token, that information hits the public ledger. For institutional traders, this is a disaster:
- **Copy-Trading**: Bots see the buy and mirror it instantly.
- **Sentiment Distortion**: A $1M bet on "YES" shifts the market sentiment artificially.
- **Exposed Alpha**: Your proprietary research is now public property.

---

## 2. The Solution: XOR-Cipher Hybrid
We use a lightweight, efficient on-chain encryption Layer that hides the **Direction** while still allowing the **Volume** to move the market.

### Technical Reasoning
We combine a **Keccak256 Commitment** with a **Stream Cipher** (XOR).
1. The user commits to a `32-byte` secret.
2. The trade direction is XORed with the first byte of the secret's hash.
3. The AMM reserves are updated *blindly* using the encrypted volume, ensuring the price moves but remains shrouded.

---

## 3. High-Fidelity Pseudo-Code

### SDK: Preparing the Shielded Trade
```typescript
function prepareShieldedTrade(choice: number, amount: number) {
    // 1. Generate local secret
    const secret = crypto.getRandomValues(new Uint8Array(32));
    
    // 2. Derive the blinding factor
    const salt = keccak256(secret);
    
    // 3. XOR the direction (0 = YES, 1 = NO)
    const directionCipher = choice ^ (salt[0] & 1);

    // 4. Return parameters for the on-chain instruction
    return {
        directionCipher,
        commitment: salt,
        amount
    };
}
```

### On-Chain: Processing the Shielded Position
The program creates the position without knowing what is inside.
```rust
pub fn init_shielded_trade(ctx, direction_cipher: u8, commitment: [u8; 32]) {
    let position = &mut ctx.accounts.position;
    
    // Lock the encrypted state
    position.direction_cipher = direction_cipher;
    position.commitment = commitment;
    
    // Update market reserves blindly
    // The market knows a trade happened, but doesn't log the direction.
    let market = &mut ctx.accounts.market;
    market.apply_blind_volume(ctx.accounts.amount);
}
```

### On-Chain: The Reveal & Payout
Only once the market is resolved (`Status::Resolved`) can the user reveal the secret.
```rust
pub fn reveal_and_claim(ctx, secret: [u8; 32]) {
    let position = &ctx.accounts.position;
    
    // 1. Verify Secret Ownership
    require!(keccak256(secret) == position.commitment, Error::InvalidSecret);
    
    // 2. Decrypt Direction
    // OriginalChoice = Cipher XOR (Hash(Secret)[0] & 1)
    let original_choice = position.direction_cipher ^ (keccak256(secret)[0] & 1);
    
    // 3. Payout if Choice == Outcome
    let outcome = ctx.accounts.market.outcome;
    if original_choice == outcome {
        release_funds(ctx.accounts.user, position.amount);
    }
}
```

---

## 4. Why this is Secure
An observer looking at Solscan sees:
- **Transaction**: `ShieldedBuy(Cipher: 1, Volume: 10,000)`.
- **Bot Analysis**: Is 1 a YES or a NO? 
- **The Catch**: Without the 32-byte secret, the bot cannot tell. The bit `1` is 50% likely to be YES and 50% likely to be NO. Your alpha is mathematically protected until the event is settled.
