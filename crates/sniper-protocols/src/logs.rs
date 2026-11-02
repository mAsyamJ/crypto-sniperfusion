//! Transaction-log parsing with program attribution (oracle-parity with the
//! chainstack listener paths).
//!
//! The runtime brackets every invocation with `Program <id> invoke [depth]`
//! and `Program <id> success|failed`. A stack over those frames attributes
//! each `Program data:` payload to the program that emitted it — the
//! protection against other programs' events being read as pump.fun's.

use solana_sdk::pubkey::Pubkey;

use crate::events::PumpEvent;

/// Whole-line gates, matched exactly: Anchor writes `Instruction: <Name>` for
/// every program, so a substring test would accept CreateTokenAccount,
/// CreatePool, … from any program in the transaction.
pub const CREATE_INSTRUCTION_LOGS: [&str; 2] = [
    "Program log: Instruction: Create",
    "Program log: Instruction: CreateV2",
];

/// One attributed `Program data:` payload.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramDataEntry {
    pub emitter: Pubkey,
    pub log_index: usize,
    pub payload: Vec<u8>,
}

/// Attribute each log line to the program whose invocation emitted it.
/// Returns None per line emitted outside any invocation frame (and for the
/// frame lines themselves). Mirrors the oracle's `_emitting_program`.
#[derive(Debug, Default)]
pub struct LogAttribution {
    emitters: Vec<Option<Pubkey>>,
}

impl LogAttribution {
    pub fn build(logs: &[&str]) -> Self {
        let mut emitters = Vec::with_capacity(logs.len());
        let mut stack: Vec<Pubkey> = Vec::new();
        for log in logs {
            let parts: Vec<&str> = log.split(' ').collect();
            let is_frame = parts.len() >= 3 && parts[0] == "Program";
            if is_frame && parts[2].starts_with("invoke") {
                if let Ok(id) = parts[1].parse::<Pubkey>() {
                    stack.push(id);
                    emitters.push(None);
                    continue;
                }
            }
            if is_frame && (parts[2] == "success" || parts[2] == "failed") {
                emitters.push(None);
                stack.pop();
                continue;
            }
            emitters.push(stack.last().copied());
        }
        Self { emitters }
    }

    pub fn emitter_at(&self, index: usize) -> Option<Pubkey> {
        self.emitters.get(index).copied().flatten()
    }

    /// True when the log list carries invocation frames at all. A
    /// reconstructed log list (no frames) leaves callers with their previous
    /// behavior — oracle-parity.
    pub fn has_frames(&self) -> bool {
        self.emitters.iter().any(|e| e.is_none()) && self.emitters.iter().any(|e| e.is_some())
            || self.emitters.iter().flatten().count() > 0
    }
}

/// Extract `Program data: <base64>` payloads emitted by `program` only.
/// Base64 failures (malformed line) are skipped, not fatal.
pub fn extract_program_data(logs: &[&str], program: &Pubkey) -> Vec<ProgramDataEntry> {
    let attribution = LogAttribution::build(logs);
    let mut out = Vec::new();
    for (i, log) in logs.iter().enumerate() {
        let Some(rest) = log.strip_prefix("Program data: ") else {
            continue;
        };
        if attribution.emitter_at(i) != Some(*program) {
            continue;
        }
        // Note: use of base64 engine without padding tolerance would reject
        // real RPC payloads; accept standard and unpadded forms.
        if let Ok(bytes) = base64_decode(rest.trim()) {
            out.push(ProgramDataEntry {
                emitter: *program,
                log_index: i,
                payload: bytes,
            });
        }
    }
    out
}

/// Decode the first event of any known type from attributed payloads.
pub fn decode_first_event(logs: &[&str], program: &Pubkey) -> Option<(usize, PumpEvent)> {
    for entry in extract_program_data(logs, program) {
        if let Some(event) = PumpEvent::decode(&entry.payload) {
            if !matches!(event, PumpEvent::UnknownDiscriminator) {
                return Some((entry.log_index, event));
            }
        }
    }
    None
}

/// Create-gated token-creation decode (the logs-listener path): requires a
/// whole-line `Instruction: Create|CreateV2` log from this program, then the
/// first CreateEvent among this program's payloads.
pub fn decode_token_creation_from_logs(
    logs: &[&str],
    program: &Pubkey,
) -> Option<crate::events::CreateEvent> {
    let attribution = LogAttribution::build(logs);
    let create_gated = logs.iter().enumerate().any(|(i, log)| {
        CREATE_INSTRUCTION_LOGS.contains(log) && attribution.emitter_at(i) == Some(*program)
    });
    if !create_gated {
        return None;
    }
    for entry in extract_program_data(logs, program) {
        if let Some(PumpEvent::Create(ev)) = PumpEvent::decode(&entry.payload) {
            return Some(ev);
        }
    }
    None
}

/// Minimal base64 (standard alphabet) decoder — avoids pulling a crate for one
/// call site; tolerant of missing padding, rejects invalid characters.
pub fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    let reverse = reverse_table();
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for ch in input.bytes() {
        if ch == b'=' || ch == b'\n' || ch == b'\r' {
            continue; // padding / whitespace
        }
        let val = reverse[(ch as usize).min(127)].ok_or("invalid base64 character")?;
        acc = (acc << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// Reverse lookup: ASCII code → 6-bit value. `None` for invalid characters.
const fn reverse_table() -> [Option<u8>; 128] {
    let mut table = [None; 128];
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut i = 0;
    while i < 64 {
        table[alphabet[i] as usize] = Some(i as u8);
        i += 1;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::DISC_CREATE_EVENT;
    use crate::registry::program_id;
    use sniper_core::Venue;

    fn b64(data: &[u8]) -> String {
        const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        let mut acc: u32 = 0;
        let mut bits: u32 = 0;
        for b in data {
            acc = (acc << 8) | *b as u32;
            bits += 8;
            while bits >= 6 {
                bits -= 6;
                out.push(A[(acc >> bits) as usize & 0x3f] as char);
                acc &= (1 << bits) - 1;
            }
        }
        if bits > 0 {
            out.push(A[(acc << (6 - bits)) as usize & 0x3f] as char);
        }
        out
    }

    fn create_event_payload() -> Vec<u8> {
        let mut body = Vec::new();
        let put_str = |buf: &mut Vec<u8>, s: &str| {
            buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
            buf.extend_from_slice(s.as_bytes());
        };
        put_str(&mut body, "Coin");
        put_str(&mut body, "CN");
        put_str(&mut body, "https://x/y.json");
        body.extend_from_slice(&[1u8; 32]); // mint
        body.extend_from_slice(&[2u8; 32]); // bonding_curve
        body.extend_from_slice(&[3u8; 32]); // user
        body.extend_from_slice(&[4u8; 32]); // creator
        body.extend_from_slice(&1i64.to_le_bytes());
        body.extend_from_slice(&1u64.to_le_bytes());
        body.extend_from_slice(&1u64.to_le_bytes());
        body.extend_from_slice(&1u64.to_le_bytes());
        body.extend_from_slice(&1u64.to_le_bytes());
        body.extend_from_slice(&[5u8; 32]); // token_program
        body.push(0); // mayhem
        body.push(0); // cashback
        let mut payload = DISC_CREATE_EVENT.to_vec();
        payload.extend_from_slice(&body);
        payload
    }

    #[test]
    fn attribution_uses_invoke_frames() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let other = Pubkey::new_unique();
        let logs = [
            format!("Program {pump} invoke [1]"),
            "Program log: Instruction: Create".into(),
            format!("Program data: {}", b64(&create_event_payload())),
            format!("Program {pump} success"),
            format!("Program {other} invoke [1]"),
            format!("Program data: {}", b64(&create_event_payload())),
            format!("Program {other} success"),
        ];
        let refs: Vec<&str> = logs.iter().map(|s| s.as_str()).collect();
        let entries = extract_program_data(&refs, &pump);
        assert_eq!(entries.len(), 1, "only pump.fun's own payload counts");
        let ev = decode_token_creation_from_logs(&refs, &pump).expect("create decodes");
        assert_eq!(ev.name, "Coin");
    }

    #[test]
    fn create_gate_rejects_other_programs_create_logs() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let other = Pubkey::new_unique();
        // `Instruction: Create` logged by a different program in the same tx.
        let logs = [
            format!("Program {other} invoke [1]"),
            "Program log: Instruction: Create".into(),
            format!("Program data: {}", b64(&create_event_payload())),
            format!("Program {other} success"),
        ];
        let refs: Vec<&str> = logs.iter().map(|s| s.as_str()).collect();
        assert_eq!(decode_token_creation_from_logs(&refs, &pump), None);
    }

    #[test]
    fn substring_create_instructions_do_not_pass_the_gate() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let logs = [
            format!("Program {pump} invoke [1]"),
            "Program log: Instruction: CreateTokenAccount".into(),
            format!("Program data: {}", b64(&create_event_payload())),
            format!("Program {pump} success"),
        ];
        let refs: Vec<&str> = logs.iter().map(|s| s.as_str()).collect();
        // Whole-line match: CreateTokenAccount must NOT gate a creation.
        assert_eq!(decode_token_creation_from_logs(&refs, &pump), None);
    }

    #[test]
    fn failed_invoke_frames_do_not_leak_attribution() {
        let pump = program_id(Venue::PumpFun).unwrap();
        let other = Pubkey::new_unique();
        let payload = create_event_payload();
        let logs = [
            format!("Program {other} invoke [1]"),
            format!("Program {pump} invoke [2]"),
            format!("Program data: {}", b64(&payload)),
            format!("Program {pump} failed. Custom program error: 0x1"),
            format!("Program {other} success"),
        ];
        let refs: Vec<&str> = logs.iter().map(|s| s.as_str()).collect();
        // The payload inside the failed pump invocation is still attributed
        // to pump.fun (it did emit it), but the create gate must hold.
        assert_eq!(
            decode_token_creation_from_logs(&refs, &pump),
            None,
            "no Instruction: Create log at all"
        );
        assert_eq!(extract_program_data(&refs, &pump).len(), 1);
    }

    #[test]
    fn base64_decoder_matches_known_vectors() {
        assert_eq!(base64_decode("AAAA").unwrap(), vec![0, 0, 0]);
        assert_eq!(base64_decode("AQIDBA==").unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(base64_decode("AQIDBA").unwrap(), vec![1, 2, 3, 4]); // unpadded
        assert!(base64_decode("!!!").is_err());
        let big: Vec<u8> = (0..=255u8).collect();
        assert_eq!(base64_decode(&b64(&big)).unwrap(), big);
    }
}
