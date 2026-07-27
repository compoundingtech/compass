//! Progress-event identity.
//!
//! Plan and Step identity are no longer minted — a Version is identified by the
//! hash of its source bytes (decision 0014) and a Step by the name it is
//! declared under (decision 0012). The only remaining minted value is a Progress
//! Event id, which is not a Plan concept: it only has to be unique within a
//! directory two machines may both write to. It carries no location and is drawn
//! from the OS entropy source so concurrent writers cannot collide.

use std::fs::File;
use std::io::Read;

/// Crockford base32 alphabet: 10 digits + 22 letters, excluding I, L, O, U so
/// the ambiguous glyph pairs cannot occur when an id is read aloud.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const SUFFIX_LEN: usize = 10;

/// Mint a fresh Progress Event id (`ev_…`).
pub fn event_id() -> Result<String, String> {
    let bytes = entropy(8)?;
    Ok(format!("ev_{}", encode_crockford(&bytes)))
}

fn encode_crockford(bytes: &[u8]) -> String {
    let mut acc: u64 = 0;
    for b in bytes.iter().take(8) {
        acc = (acc << 8) | u64::from(*b);
    }
    let mut out = [0u8; SUFFIX_LEN];
    for slot in out.iter_mut().rev() {
        *slot = CROCKFORD[(acc & 0x1f) as usize];
        acc >>= 5;
    }
    String::from_utf8(out.to_vec()).expect("crockford alphabet is ASCII")
}

fn entropy(n: usize) -> Result<Vec<u8>, String> {
    let mut f = File::open("/dev/urandom")
        .map_err(|e| format!("cannot open /dev/urandom to mint an event id: {e}"))?;
    let mut buf = vec![0u8; n];
    f.read_exact(&mut buf)
        .map_err(|e| format!("cannot read /dev/urandom to mint an event id: {e}"))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn event_ids_are_well_formed_and_unique() {
        let mut seen = HashSet::new();
        for _ in 0..2000 {
            let id = event_id().unwrap();
            assert!(id.starts_with("ev_"));
            assert_eq!(id.len(), 3 + SUFFIX_LEN);
            assert!(seen.insert(id), "duplicate id");
        }
    }

    #[test]
    fn encoding_avoids_ambiguous_glyphs() {
        assert_eq!(encode_crockford(&[0u8; 8]), "0000000000");
        assert_eq!(encode_crockford(&[0xff; 8]), "ZZZZZZZZZZ");
        for _ in 0..500 {
            let id = event_id().unwrap();
            for bad in ['I', 'L', 'O', 'U'] {
                assert!(!id.contains(bad));
            }
        }
    }
}
