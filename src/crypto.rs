//! Keyed format v1. Keys are 32 random bytes, represented as 64 hex digits.
//! AES-SIV deliberately uses deterministic encryption; no random nonce is needed.
use aes_siv::{aead::KeyInit, siv::Aes256Siv};
use hmac::{Hmac, Mac};
use sha2::Sha512;
use zeroize::Zeroizing;

// Never implement Debug or Serialize: the secret must not enter exports or errors.
pub struct MagicKey(Zeroizing<[u8; 32]>);
impl MagicKey {
    pub fn from_hex(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Magic key must be exactly 64 hexadecimal digits (32 bytes); use a generated random key, not a password".into());
        }
        let mut bytes = Zeroizing::new([0; 32]);
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap();
        }
        Ok(Self(bytes))
    }
    fn derive(&self, domain: &[u8], message: &[u8]) -> Zeroizing<[u8; 64]> {
        let mut mac = <Hmac<Sha512> as Mac>::new_from_slice(self.0.as_ref()).unwrap();
        mac.update(b"Code-Is-Magic/keyed/v1\0");
        mac.update(&(domain.len() as u64).to_be_bytes());
        mac.update(domain);
        mac.update(message);
        Zeroizing::new(mac.finalize().into_bytes().into())
    }
    pub(crate) fn seed(&self, domain: &[u8], message: &[u8]) -> u64 {
        u64::from_be_bytes(self.derive(domain, message)[..8].try_into().unwrap())
    }
    pub(crate) fn seal(&self, domain: &[u8], bytes: &[u8]) -> Result<Vec<u8>, String> {
        let derived = self.derive(domain, b"encryption");
        Aes256Siv::new_from_slice(derived.as_ref())
            .unwrap()
            .encrypt([domain], bytes)
            .map_err(|_| "Could not encrypt magic payload".into())
    }
    pub(crate) fn open(&self, domain: &[u8], bytes: &[u8]) -> Result<Vec<u8>, String> {
        let derived = self.derive(domain, b"encryption");
        Aes256Siv::new_from_slice(derived.as_ref())
            .unwrap()
            .decrypt([domain], bytes)
            .map_err(|_| "Wrong magic key or damaged encrypted payload".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_independent_python_aessiv_vector() {
        // Cross-checked with cryptography.AESSIV and Python hmac/sha512.
        // Synthetic test key; not a user secret.
        let key = MagicKey::from_hex(&"01".repeat(32)).unwrap();
        let encrypted = key
            .seal(b"name-contour/v2", "魔法::cast".as_bytes())
            .unwrap();
        let hex: String = encrypted.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "d7bc3f55256491415aca6d18a4a239365e10cc2f0899793334ffc4c6"
        );
        assert_eq!(
            key.open(b"name-contour/v2", &encrypted).unwrap(),
            "魔法::cast".as_bytes()
        );
        assert!(key.open(b"source-capsule/v1", &encrypted).is_err());
    }
}
