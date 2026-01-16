//! Instruction handlers for the prediction market protocol
//!
//! Each instruction represents an action users can take:
//! - `initialize` - Set up the protocol (admin only, once)
//! - `create_market` - Create a new prediction market (permissionless)
//! - `trade` - Buy/sell outcome tokens
//! - `resolve` - Determine the winning outcome (oracle only)
//! - `redeem` - Claim winnings after resolution

pub mod initialize;
pub mod create_market;
pub mod trade;
pub mod resolve;
pub mod redeem;

pub use initialize::*;
pub use create_market::*;
pub use trade::*;
pub use resolve::*;
pub use redeem::*;
