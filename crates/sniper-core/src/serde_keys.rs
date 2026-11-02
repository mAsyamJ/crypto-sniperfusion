//! Human-readable serde for Solana key types: base58 strings on the wire
//! instead of raw byte arrays (the solana-sdk 2.x default). Fixtures and
//! journals stay hand-reviewable.

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

pub mod pubkey_str {
    use super::*;
    use serde::Deserialize as _;

    pub fn serialize<S: serde::Serializer>(key: &Pubkey, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&key.to_string())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Pubkey, D::Error> {
        let text = String::deserialize(d)?;
        text.parse::<Pubkey>()
            .map_err(|e| serde::de::Error::custom(format!("invalid base58 pubkey {text:?}: {e}")))
    }
}

pub mod signature_str {
    use super::*;
    use serde::Deserialize as _;

    pub fn serialize<S: serde::Serializer>(sig: &Signature, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&bs58::encode(sig.as_ref()).into_string())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Signature, D::Error> {
        let text = String::deserialize(d)?;
        let bytes = bs58::decode(&text)
            .into_vec()
            .map_err(serde::de::Error::custom)?;
        let arr: [u8; 64] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("signature must decode to 64 bytes"))?;
        Ok(Signature::from(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Roundtrip {
        #[serde(with = "pubkey_str")]
        key: Pubkey,
        #[serde(with = "signature_str")]
        sig: Signature,
    }

    #[test]
    fn base58_roundtrip() {
        let v = Roundtrip {
            key: "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
                .parse()
                .unwrap(),
            sig: Signature::from([7u8; 64]),
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"));
        let back: Roundtrip = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn rejects_malformed_keys() {
        assert!(serde_json::from_str::<Roundtrip>("{\"key\":\"!!\",\"sig\":\"AA\"}").is_err());
    }
}
