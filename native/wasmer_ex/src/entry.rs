//! Simple Rust representations of an on-chain entry and its header.
//!
//! * All fixed-length hashes are `[u8; 32]` (SHA-256 style).
//! * The signer is a 48-byte BLS public-key.
//! * `vr` and the body are variable-length byte vectors.
use eetf::Term;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use std::collections::HashMap;

/// 32-byte hash.
pub type Hash32 = [u8; 32];

/// 48-byte BLS public key.
pub type BlsPk = [u8; 48];

/// Slot (time-slot / height analogue).
pub type Slot = u64;

/// Header section found inside every entry.
///
/// ```text
///               +--------------+
///               | prev_hash    |----.
///               +--------------+    |
///               | prev_slot    |    |  used for
///               +--------------+    |  fork-choice /
///               | height       |    |  chain-sync
///               +--------------+    |
///               | slot         |    |
///               +--------------+    |
///               | txs_hash     |----'  (Merkle root)
///               +--------------+
///               | dr           |  deterministic randomness material
///               +--------------+
///               | vr           |  VRF proof
///               +--------------+
///               | signer       |  producer PK
///               +--------------+
///               | sig          |  signature *(if you store it externally)*
///               +--------------+
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub dr: Hash32,
    pub height: u64,
    pub prev_hash: Hash32,
    pub prev_slot: Slot,
    pub signer: BlsPk,
    pub slot: Slot,
    pub txs_hash: Hash32,
    pub vr: Vec<u8>, // could be [u8; ?] if length is fixed
}

/// A complete entry as it sits in RocksDB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub header: Header,

    /// Raw body (transactions, receipts…) – still ETF-encoded unless you
    /// parse it further.
    pub body: Vec<u8>,
}


impl TryFrom<&Term> for Header {
    type Error = String;

    fn try_from(term: &Term) -> Result<Self, Self::Error> {
        let m = match term {
            Term::Map(m) => &m.map,
            _            => return Err("header is not a map".into()),
        };

        // helper to grab a binary field and enforce its size
        fn bin<const N: usize>(m: &std::collections::HashMap<Term, Term>, name: &str)
            -> Result<[u8; N], String>
        {
            let key = Term::Atom(name.into());
            let Term::Binary(b) = m.get(&key).ok_or_else(|| format!("{name} missing"))? else {
                return Err(format!("{name} not binary"));
            };
            b.bytes[..N].try_into().map_err(|_| format!("{name} wrong length"))
        }

        Ok(Header {
            dr:        bin::<32>(m, "dr")?,
            height:    get_u64(m, "height")?,
            prev_hash: bin::<32>(m, "prev_hash")?,
            prev_slot: get_u64(m, "prev_slot")?,
            signer:    bin::<48>(m, "signer")?,
            slot:      get_u64(m, "slot")?,
            txs_hash:  bin::<32>(m, "txs_hash")?,
            vr:        get_vec(m, "vr")?,
        })
    }
}

/// Extract an arbitrary-length byte vector (`Vec<u8>`) from a map field
/// that is either an ETF *Binary* or a list of byte integers.
pub fn get_vec(
    m: &HashMap<Term, Term>,
    name: &str,
) -> Result<Vec<u8>, String> {
    let key = Term::Atom(name.into());
    match m.get(&key) {
        Some(Term::Binary(bin)) => Ok(bin.bytes.clone()),
        Some(Term::List(list)) => {
            // Each element must be a SmallInteger (0..=255)
            let mut v = Vec::with_capacity(list.elements.len());
            for term in &list.elements {
                match term {
                    Term::FixInteger(b) => v.push(b.value.try_into().unwrap()),
                    other => return Err(format!("{name} list element not byte: {other:?}")),
                }
            }
            Ok(v)
        }
        other => Err(format!("{name} missing or wrong type: {other:?}")),
    }
}

/// Extract a `u64` from a map field that can be an ETF
/// *SmallInteger*, *Integer* or *BigInteger*.
pub fn get_u64(m: &HashMap<Term, Term>, name: &str) -> Result<u64, String> {
    let key = Term::Atom(name.into());
    match m.get(&key) {
        Some(Term::FixInteger(n)) => Ok(n.value.try_into().unwrap()),
        Some(Term::BigInteger(bi)) => {
            // cheapest way is via num-bigint

            bi.value
                .to_u64()
                .ok_or_else(|| format!("{name} too large for u64"))
        }
        other => Err(format!("{name} missing or not an integer: {other:?}")),
    }
}
