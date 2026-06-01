use crate::xor::{xor_bytes, XOR_KEY};

include!(concat!(env!("OUT_DIR"), "/embedded_key.rs"));

/// Reassemble the API key at runtime from the obfuscated bytes embedded at build time.
pub fn api_key() -> String {
    String::from_utf8(xor_bytes(OBFUSCATED_KEY, XOR_KEY)).unwrap_or_default()
}
