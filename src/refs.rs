//! Minted opaque references (CMP-R07, decision 0004, DQ04).
//!
//! A `PlanRef` or `StepRef` is minted at creation and never derived from
//! content, so rewording a step preserves its identity. Minting must not
//! collide when two machines mint concurrently and neither can reach a
//! coordinator, so the suffix is drawn from the OS entropy source.
//!
//! DQ04 is open on width and whether to encode the minting host. This
//! implementation encodes no host — a ref must carry no location (CMP-R07) —
//! and uses 50 bits of entropy rendered as 10 Crockford base32 characters.
//! Crockford is chosen because refs appear in prose and get read aloud: it
//! omits I, L, O and U, so the ambiguous glyph pairs cannot occur.

use std::fmt;
use std::fs::File;
use std::io::Read;

/// Crockford base32 alphabet: 10 digits + 22 letters, excluding I, L, O, U.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Number of base32 characters after the type prefix.
const SUFFIX_LEN: usize = 10;

/// The kind of entity a ref names. The prefix makes a ref self-describing in
/// prose and in error messages without a lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Plan,
    Step,
    /// Progress Event identity. Not a Plan concept — it only has to be unique
    /// within a directory two machines may both write to.
    Event,
}

impl RefKind {
    pub fn prefix(self) -> &'static str {
        match self {
            RefKind::Plan => "pl_",
            RefKind::Step => "st_",
            RefKind::Event => "ev_",
        }
    }
}

impl fmt::Display for RefKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefKind::Plan => write!(f, "plan"),
            RefKind::Step => write!(f, "step"),
            RefKind::Event => write!(f, "event"),
        }
    }
}

/// Mint a fresh opaque reference of the given kind.
pub fn mint(kind: RefKind) -> Result<String, String> {
    let bytes = entropy(8)?;
    Ok(format!("{}{}", kind.prefix(), encode_crockford(&bytes)))
}

/// Render the low 50 bits of `bytes` as 10 Crockford base32 characters.
fn encode_crockford(bytes: &[u8]) -> String {
    let mut acc: u64 = 0;
    for b in bytes.iter().take(8) {
        acc = (acc << 8) | u64::from(*b);
    }
    let mut out = [0u8; SUFFIX_LEN];
    // Fill right-to-left so the whole 50-bit space is used uniformly.
    for slot in out.iter_mut().rev() {
        *slot = CROCKFORD[(acc & 0x1f) as usize];
        acc >>= 5;
    }
    String::from_utf8(out.to_vec()).expect("crockford alphabet is ASCII")
}

/// Read `n` bytes from the OS entropy source.
fn entropy(n: usize) -> Result<Vec<u8>, String> {
    let mut f = File::open("/dev/urandom")
        .map_err(|e| format!("cannot open /dev/urandom to mint a reference: {e}"))?;
    let mut buf = vec![0u8; n];
    f.read_exact(&mut buf)
        .map_err(|e| format!("cannot read /dev/urandom to mint a reference: {e}"))?;
    Ok(buf)
}

/// Whether `s` is well-formed as a ref of the given kind. Used to reject
/// nonsense on the command line before it reaches authored content.
pub fn is_valid(s: &str, kind: RefKind) -> bool {
    let Some(rest) = s.strip_prefix(kind.prefix()) else {
        return false;
    };
    rest.len() == SUFFIX_LEN && rest.bytes().all(|b| CROCKFORD.contains(&b))
}

/// Whether `s` looks like a ref of any known kind.
pub fn is_any_ref(s: &str) -> bool {
    is_valid(s, RefKind::Plan) || is_valid(s, RefKind::Step)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn minted_refs_are_well_formed() {
        let p = mint(RefKind::Plan).unwrap();
        assert!(p.starts_with("pl_"), "{p}");
        assert_eq!(p.len(), 3 + SUFFIX_LEN);
        assert!(is_valid(&p, RefKind::Plan));
        assert!(!is_valid(&p, RefKind::Step));

        let s = mint(RefKind::Step).unwrap();
        assert!(is_valid(&s, RefKind::Step));
        assert!(!is_valid(&s, RefKind::Plan));
    }

    #[test]
    fn minted_refs_do_not_repeat() {
        // Not a collision proof, only a smoke test that entropy is actually
        // being read rather than a constant returned.
        let mut seen = HashSet::new();
        for _ in 0..2000 {
            assert!(seen.insert(mint(RefKind::Step).unwrap()), "duplicate ref");
        }
    }

    #[test]
    fn encoding_avoids_ambiguous_glyphs() {
        for _ in 0..500 {
            let r = mint(RefKind::Plan).unwrap();
            let suffix = &r[3..];
            for bad in ['I', 'L', 'O', 'U'] {
                assert!(!suffix.contains(bad), "{r} contains {bad}");
            }
        }
    }

    #[test]
    fn encoding_is_deterministic_for_fixed_input() {
        assert_eq!(encode_crockford(&[0u8; 8]), "0000000000");
        assert_eq!(encode_crockford(&[0xff; 8]), "ZZZZZZZZZZ");
    }

    #[test]
    fn rejects_malformed_refs() {
        assert!(!is_valid("pl_", RefKind::Plan));
        assert!(!is_valid("pl_short", RefKind::Plan));
        assert!(!is_valid("pl_IIIIIIIIII", RefKind::Plan)); // I is not in the alphabet
        assert!(!is_valid("nope", RefKind::Plan));
        assert!(is_any_ref("st_0123456789"));
    }
}
