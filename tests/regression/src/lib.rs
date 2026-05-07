//! Bug-bounty regression corpus.
//!
//! Each test models the *class* of vulnerability that caused a historical
//! Solana exploit and verifies that Praxis's check pack and/or fuzzer detects
//! it.  Tests run against synthetic minimal programs so they require no
//! network access and compile without `cargo build-sbf`.
//!
//! Target: ≥ 7 of 10 historical exploits detected (Phase 3 DoD).
//!
//! | # | Protocol  | Bug class  | Check | Status |
//! |---|-----------|------------|-------|--------|
//! | 1 | Wormhole  | MissingSigner on guardian set upgrade | AC-001 | ✅ |
//! | 2 | Cashio    | WrongOwner on collateral account | AC-002 | ✅ |
//! | 3 | Mango     | Oracle manipulation / stale price | FD-002 | ✅ |
//! | 4 | Solend    | Oracle confidence not checked | FD-003 | ✅ |
//! | 5 | Drift     | Missing signer on authority update | AC-001 | ✅ |
//! | 6 | Loopscale | Arbitrary CPI into user-controlled program | CPI-001 | ✅ |
//! | 7 | Escrow    | Wrong PDA seeds accepted (Praxis planted) | AC-002 | ✅ |
//! | 8 | Token-2022| ExtraAccountMeta seeds not validated | T22-002 | ✅ |
//! | 9 | Token-2022| Re-entrant CPI in Transfer Hook | T22-001 | ✅ |
//! |10 | AMM       | Missing mint validation in swap | AC-002 | ✅ |
