# Technical Explanation: Selective Compliance (Auditor Keys)

## 1. The Problem: "The Compliance Gap"
Full privacy is usually a signal of high risk for regulators and institutional risk committees.
- **The Issue**: If you trade on a 100% anonymized "Black Box," you cannot prove the source of funds or verify that the trade was legal (Lifting the corporate veil).
- **The Friction**: This prevents billions in capital from entering DeFi because professional entities cannot audit their own activities.

---

## 2. The Solution: Auditor View Keys
We provide **Selective Visibility**. The user holds the "Master Key" to their data, but can mint "Viewing Keys" to safely share that data with specific third parties (Auditors, Tax authorities, LPs).

### Technical Reasoning
We use **Metadata Encryption** centered around a view-key hash.
1. The trade details are encrypted using a symmetric key.
2. A hash of the "View Key" is stored alongside the position.
3. Only an entity providing the literal string (the View Key) can decrypt the metadata.

---

## 3. High-Fidelity Pseudo-Code

### The Secure Metadata Schema
```rust
struct PositionMetadata {
    // Encrypted blob containing: [EntryPrice, Volume, Direction, Timestamp]
    encrypted_blob: [u8; 256],
    
    // The "Audit Lock"
    view_key_hash: [u8; 32],
}
```

### The Auditor Handshake (Program Logic)
An auditor uses the View Key (provided by the user) to "Retrieve the Truth."

```rust
pub fn audit_position(ctx, view_key: [u8; 32]) -> Result<AuditReport> {
    let position = &ctx.accounts.position;

    // 1. Authenticate the Auditor
    // We check if Keccak256(view_key) == the stored hash.
    require!(keccak::hash(view_key).0 == position.view_key_hash, Error::AccessDenied);

    // 2. Decrypt the Data
    // The program decrypts the details ONLY for this specific audit call.
    let details = crypto::decrypt(position.encrypted_blob, view_key);

    // 3. Return a Verified Report
    // This allows the Auditor to generate a CSV/Proof of Trade.
    return AuditReport {
        verified_direction: details.direction,
        verified_price: details.price,
        verified_owner: details.trader_pubkey,
    };
}
```

---

## 4. Why this is Secure
- **The Public**: Sees nothing but the `view_key_hash`. Without the original key, they cannot brute-force the metadata.
- **The Auditor**: Can verify the trade with 100% accuracy, matching the user's reported profit/loss with the on-chain reality.
- **The User**: Maintains full control. They only "lift the veil" when they choose to, and only for the person they choose. This is the **Professional Bridge** that brings institutional liquidity to Private Prediction Markets.
