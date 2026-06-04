pub const XOR_KEY: &[u8] = b"kg-memphis-2026-salt-do-not-reuse";

/// XOR each byte of `data` with a repeating key. Symmetric: applying twice restores input.
pub fn xor_bytes(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter().enumerate().map(|(i, b)| b ^ key[i % key.len()]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_restores_original() {
        let secret = b"sk-abc123-EXAMPLE";
        let obf = xor_bytes(secret, XOR_KEY);
        assert_ne!(&obf[..], &secret[..]);
        assert_eq!(&xor_bytes(&obf, XOR_KEY)[..], &secret[..]);
    }
}
