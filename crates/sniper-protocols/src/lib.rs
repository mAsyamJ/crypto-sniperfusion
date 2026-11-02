//! sniper-protocols: venue registry and instruction-version tables.
//!
//! Every constant here is evidence-linked to `docs/PROTOCOL_COMPATIBILITY.md`
//! (source: upstream/pump-docs @ 639b87f official IDLs and instruction docs).
//! Byte-level claims (discriminators) are computed from the Anchor naming
//! convention at runtime and asserted against IDL-derived constants in tests.

pub mod amm;
pub mod builders;
pub mod curve;
pub mod events;
pub mod logs;
pub mod math;
pub mod pda;
pub mod registry;

pub use amm::PumpSwapIx;
pub use builders::{
    buy_exact_quote_in_v3, buy_v2, buy_v3, buyback_fee_recipient, global_volume_accumulator,
    pumpswap_sell_v2_complete, sell_v2, sell_v3, CurveTradeContext, CurveV2Extras,
    PumpSwapSellV2Inputs, BUYBACK_FEE_RECIPIENTS,
};
pub use curve::CurveTradeIx;
pub use events::{BorshReader, CreateEvent, PumpEvent, TradeEvent};
pub use logs::{
    decode_first_event, decode_token_creation_from_logs, extract_program_data, LogAttribution,
};
pub use math::{
    amm_buy_out, amm_sell_out, apply_fee_in, curve_buy_out, curve_sell_out,
    effective_quote_reserves, MathError,
};
pub use registry::{
    program_id, program_id_for, PROGRAM_LETS_BONK, PROGRAM_PUMP_FUN, PROGRAM_PUMP_SWAP,
};
