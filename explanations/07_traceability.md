# Technical Explanation: Traceability (Gas-Leak Identity)

## 1. The Problem: "The Funding Trace"
Privacy is only as strong as its weakest link. In 99% of cases, that link is **SOL for Gas**.
- **The Cycle**: You create a private "Ghost Wallet." 
- **The Block**: The Ghost Wallet has 0 SOL, so it cannot sign a "Claim" transaction.
- **The Leak**: You send 0.05 SOL from your Main Wallet to the Ghost Wallet.
- **The Result**: An amateur bot scans the ledger and sees the transfer. Your "secret" identity is now linked to your public identity.

---

## 2. The Solution: Meta-Transactions (Relayer Payers)
We use a **Fee-Payer Isolation** model. The Relayer pays the gas for the Ghost Wallet, and the protocol pays back the Relayer from the user's winnings.

### Technical Reasoning
We implement **Meta-Signature Authorization**.
1. The **Ghost Wallet** signs an "Intent" (a specialized data-hash) off-chain.
2. The **Relayer** (a third-party service) wraps that Intent in a transaction.
3. The **Relayer** acts as the `Signer[0]` (Payer).
4. The **Program** verifies the Ghost Wallet's intent and executes the payout.

---

## 3. High-Fidelity Pseudo-Code

### The Off-Chain Intent (User Logic)
```typescript
async function signClaimIntent(ghostWallet, amount, secret) {
    // 1. Create a hash of the intent
    const intentHash = hash("CLAIM_REQUEST", ghostWallet.pubkey, amount, nonce);

    // 2. Sign only the hash
    const signature = await ghostWallet.sign(intentHash);

    // 3. Send to Relayer Node (Off-chain)
    return { signature, intentHash, recipient: ghostWallet.pubkey };
}
```

### The On-Chain Settlement (Program Logic)
```rust
pub fn execute_meta_claim(ctx, intent_hash: [u8; 32], signature: [u8; 64]) {
    let relayer = &ctx.accounts.relayer; // pays the gas
    let ghost_wallet = &ctx.accounts.ghost_wallet;

    // 1. Verify Authentication
    // Does the signature prove the Ghost Wallet signed this hash?
    assert!(ed25519::verify(ghost_wallet.publicKey, intent_hash, signature));

    // 2. Settlement logic
    let total_winnings = 100.0;
    let gas_fee = 0.001;

    // Send money to Ghost Wallet
    transfer(vault -> ghost_wallet, total_winnings - gas_fee);

    // Send fee to Relayer
    transfer(vault -> relayer, gas_fee);
}
```

---

## 4. Why this is Secure
To the outside world:
- The **Relayer** appears to be transacting with the **Protocol**.
- The **Ghost Wallet** appears to be receiving money from the **Protocol**.
- There is **no connection** between the User's Main Wallet and this process.
The "Gas Leak" is permanently plugged, ensuring that even a sophisticated blockchain forensics firm cannot trace the wealth back to your original trading identity.
