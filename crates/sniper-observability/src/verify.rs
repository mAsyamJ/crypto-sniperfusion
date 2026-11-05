//! Receipt integrity verification (T-VER1): mirror the producer's sealing
//! rule exactly — `receipt_sha256 = sha256(canonical JSON of the receipt
//! struct with the hash field emptied)` (`crates/sniper-strategy/src/receipt.rs`).
//! The verifier re-derives the hash from the PARSED STRUCT, so any
//! tamper with a represented field breaks it.

use serde_json::Value;
use sha2::{Digest, Sha256};
use sniper_strategy::receipt::PnlRunReceiptV2;

use crate::VerifyError;

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Parse + hash-verify a `run.json` payload. `path` is used for error
/// context only.
pub fn verify_run_receipt(path: &str, raw: &str) -> Result<PnlRunReceiptV2, VerifyError> {
    let receipt: PnlRunReceiptV2 = serde_json::from_str(raw).map_err(|e| VerifyError::Parse {
        path: path.to_string(),
        reason: format!("run.json is not a PnlRunReceiptV2: {e}"),
    })?;
    if receipt.schema_version != sniper_strategy::receipt::PNL_SCHEMA_VERSION {
        return Err(VerifyError::Unsupported {
            path: path.to_string(),
            reason: format!("schema_version {} != 2", receipt.schema_version),
        });
    }
    let computed = receipt.compute_hash();
    if computed != receipt.receipt_sha256 {
        return Err(VerifyError::SourceMismatch {
            path: path.to_string(),
            expected: receipt.receipt_sha256.clone(),
            actual: computed,
        });
    }
    Ok(receipt)
}

/// `dataset_sha256` is a hash only when it matches the 64-hex pattern;
/// otherwise it is a free-text provenance label (D-2) and passes through.
pub fn dataset_sha_is_hash(receipt: &PnlRunReceiptV2) -> bool {
    is_hash_like(&receipt.dataset_sha256)
}

fn is_hash_like(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// H8 registry structural chain check (T-REG1): each record's `prev_hash`
/// must equal the previous record's `record_sha256` (GENESIS starts).
/// `record_sha256` recomputation stays with the Python producer (its own
/// canonicalization); linkage is what the mirror verifies.
pub fn verify_registry_chain(records: &[Value]) -> Result<(), VerifyError> {
    let mut prev = "GENESIS".to_string();
    for (i, rec) in records.iter().enumerate() {
        let link = rec.get("prev_hash").and_then(|v| v.as_str()).unwrap_or("");
        if link != prev {
            return Err(VerifyError::SourceMismatch {
                path: format!("registry record {i}"),
                expected: prev,
                actual: link.to_string(),
            });
        }
        prev = rec
            .get("record_sha256")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
    }
    Ok(())
}
