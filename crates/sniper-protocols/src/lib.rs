//! sniper-protocols: venue registry and instruction-version tables.
//!
//! Every constant here is evidence-linked to `docs/PROTOCOL_COMPATIBILITY.md`
//! (source: upstream/pump-docs @ 639b87f official IDLs and instruction docs).
//! Byte-level claims (discriminators) are computed from the Anchor naming
//! convention at runtime and asserted against IDL-derived constants in tests.

pub mod amm;
pub mod curve;
pub mod registry;

pub use amm::PumpSwapIx;
pub use curve::CurveTradeIx;
pub use registry::{
    program_id, program_id_for, PROGRAM_LETS_BONK, PROGRAM_PUMP_FUN, PROGRAM_PUMP_SWAP,
};
