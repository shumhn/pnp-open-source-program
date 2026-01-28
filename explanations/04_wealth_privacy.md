# Technical Explanation: Wealth Privacy (The Crux)

## 1. The Problem: "Wealth Doxing" & The Gas Leak
On a public blockchain, transparency is the enemy of net worth. Even if your trade is private, standard payout models create two fatal links:

1.  **The Transaction Link**: When the protocol sends $100k to your wallet, the explorer shows `Protocol -> Wallet A`. Everyone knows Wallet A just got rich.
2.  **The Gas Link**: If you try to use a "Fresh Wallet B" to receive funds, you need 0.000005 SOL to pay for the gas. If you fund that wallet from your Main Wallet, the accounts are linked forever. 

**Conclusion**: To have true wealth privacy, you must break the link between **Trading Identity** (Wallet A) and **Banking Identity** (Wallet B).

---

## 2. The Solution: Destination Wallet Retrieval
We implement a **Knowledge-based Retrieval** system. Instead of the protocol "pushing" money to a known address, the money is moved into a **Shielded Vault** that is locked by a mathematical secret.

### The Innovation
- **Wallet A** performs the trade.
- **Protocol Vault** holds the winnings in a "blind" state.
- **Wallet B** (completely fresh) discovers and "retrieves" the money using a secret.
- **Relayer** pays the gas for Wallet B, severing the final link.

---

## 3. High-Fidelity Pseudo-Code

### Phase A: The "Hush" (Initiated by Wallet A)
Before claiming, the trader generates a secret and a commitment.

```typescript
// FRONTEND / SDK
async function lockWinnings(walletA, amount) {
    // 1. Generate deep-entropy secret
    const secret = crypto.getRandomValues(new Uint8Array(32));
    
    // 2. Define our "Bank Wallet" (Wallet B)
    const recipientAddress = freshWallet.publicKey;

    // 3. Create the Commitment (The Lock)
    // We bind the secret to the recipient address. 
    // This prevents anyone else from stealing the secret in transit.
    const commitment = keccak256(
        Buffer.concat([secret, recipientAddress.toBuffer()])
    );

    // 4. Move winnings to the "Shielded PDA"
    // The blockchain only stores the 'commitment' hash.
    await program.methods.initPrivacyClaim(commitment, amount).rpc();

    // 5. Store the secret locally in encrypted storage
    await secureStorage.save('exit_secret', secret);
}
```

### Phase B: The "Void" (On-Chain State)
The funds sit in a `PrivacyClaim` account indexed by the hash.

```rust
// SMART CONTRACT (Anchor/Rust)
#[account]
pub struct PrivacyClaim {
    pub commitment: [u8; 32],
    pub amount: u64,
    pub lock_until: i64, // Timing delay to stop wallet-timing analysis
    pub redeemed: bool,
}
```

### Phase C: The "Retrieval" (Initiated by Wallet B)
Wallet B uses a **Relayer** to pay for gas.

```typescript
// RELAYER / SDK
async function retrievalHandshake(walletB, relayer) {
    const savedSecret = await secureStorage.load('exit_secret');

    // Wallet B signs a proof of intent to receive the funds
    const claimTX = await program.methods.claim(savedSecret)
        .accounts({
            claimant: relayer.publicKey, // Fee Payer (Relayer)
            recipient: walletB.publicKey,
            privacyClaim: claimPDA,
        })
        .signers([relayer]) // Relayer pays the gas!
        .rpc();
}
```

### Phase D: On-Chain Proof of Knowledge
The contract verifies the secret *without* Wallet A being involved.

```rust
// SMART CONTRACT (Claim Logic)
pub fn verify_retrieval(ctx, secret: [u8; 32]) -> Result<()> {
    let claim = &ctx.accounts.privacy_claim;
    let recipient = ctx.accounts.recipient.key();

    // Re-calculate the commitment locally
    // If Hash(Secret + WalletB) == Stored_Commitment -> Success
    let reveal_hash = keccak::hash(secret + recipient).0;
    
    require!(reveal_hash == claim.commitment, Error::InvalidProof);
    require!(!claim.redeemed, Error::AlreadyClaimed);

    // Release funds directly to Wallet B
    transfer(shielded_vault -> recipient, claim.amount);
    
    // Mark as finished
    claim.redeemed = true;
    Ok(())
}
```

---

## 4. Why this is the "Crux"

1.  **Identity Shifting**: You "disappear" as User A and "re-appear" as User B. 
2.  **No On-Chain Path**: There is not a single transaction on the blockchain where Wallet A and Wallet B are present together. 
3.  **The "Knowledge" Gate**: In standard DeFi, you claim money because you **own** the account (`ctx.accounts.signer == owner`). In Private PNP, you claim money because you **know** the secret. 

**This turns the payout process into a Zero-Knowledge proof of destination. It is the gold standard for wealth privacy.**
