//! # Automated Market Maker (AMM) Module
//!
//! This module implements the **Pythagorean Bonding Curve** for pricing
//! YES/NO tokens in prediction markets.
//!
//! ## The Pythagorean AMM
//!
//! Unlike traditional constant-product AMMs (x * y = k), prediction markets
//! use a Pythagorean invariant that naturally represents probabilities:
//!
//! ```text
//!            R = √(YES² + NO²)
//!
//!   ┌────────────────────────────────────────┐
//!   │           Probability Space            │
//!   │                                         │
//!   │    NO ▲                                │
//!   │       │      ╭──────╮                  │
//!   │       │    ╱        ╲   R = constant   │
//!   │       │  ╱            ╲                │
//!   │       │╱                ╲              │
//!   │       └──────────────────▶ YES         │
//!   │                                         │
//!   │  Points on the curve = valid states    │
//!   │  Distance from origin = R (reserves)   │
//!   └────────────────────────────────────────┘
//! ```

pub mod bonding_curve;

pub use bonding_curve::*;
