//! Properties the fuzz targets assert, and the input encoding they share.
//!
//! The properties live here rather than in the targets so that plain
//! `cargo test` (stable, no sanitizer) runs them over the acceptance corpus,
//! and so that a crash found by the fuzzer reproduces as an ordinary test.
//!
//! Input encoding for `check` and `normalize`: byte 0 picks the kind (low
//! two bits, modulo 3). If its high bit is set, bytes 1..4 are `min_len`,
//! `max_len` and `max_mark_run`; otherwise the kind's default policy
//! applies. The rest is the name, which must be UTF-8. `keys` takes the
//! whole input as the name.

use restrictive_usernames::{
    Kind, Policy, Rejected, Rule, check, check_structural, is_valid, key_ci, key_cs, key_visual_ci,
    key_visual_cs, normalize,
};
use unicode_normalization::{UnicodeNormalization, is_nfc};

const KINDS: [Kind; 3] = [Kind::Username, Kind::Nickname, Kind::Label];
const CUSTOM_POLICY: u8 = 0x80;

/// Split a fuzz input into a policy and a name. `None` for input that is
/// too short or not UTF-8.
pub fn decode(data: &[u8]) -> Option<(Policy, &str)> {
    let (&head, rest) = data.split_first()?;
    let kind = KINDS[usize::from(head & 0b11) % KINDS.len()];
    let (policy, rest) = if head & CUSTOM_POLICY == 0 {
        (Policy::for_kind(kind), rest)
    } else {
        let [min, max, run, rest @ ..] = rest else {
            return None;
        };
        let policy = Policy::for_kind(kind)
            .with_min_len(u32::from(*min))
            .with_max_len(u32::from(*max))
            .with_max_mark_run(u32::from(*run));
        (policy, rest)
    };
    Some((policy, std::str::from_utf8(rest).ok()?))
}

/// The inverse of [`decode`] for a kind's default policy: a seed input.
pub fn encode(kind: Kind, name: &str) -> Vec<u8> {
    let tag = KINDS
        .iter()
        .position(|&k| k == kind)
        .expect("every kind is listed");
    let mut out = vec![tag as u8];
    out.extend_from_slice(name.as_bytes());
    out
}

fn ordinal(rule: Rule) -> usize {
    Rule::ALL
        .iter()
        .position(|r| r.as_str() == rule.as_str())
        .expect("every rule is in Rule::ALL")
}

fn assert_ordered(rules: &[Rule]) {
    assert!(
        rules.windows(2).all(|w| ordinal(w[0]) < ordinal(w[1])),
        "rules not in Rule::ALL order, or repeated: {rules:?}"
    );
}

/// `check`, `is_valid` and `check_structural` agree with each other, with
/// the policy, and with the documented reporting order.
pub fn check_properties(policy: &Policy, name: &str) {
    let full = check(name, policy);
    assert_eq!(is_valid(name, policy), full.is_empty());
    assert_ordered(&full);
    assert!(!full.contains(&Rule::NotTaken));

    let empty = name.trim().is_empty();
    assert_eq!(full.contains(&Rule::Empty), empty, "{full:?}");
    if empty {
        assert_eq!(full, [Rule::Empty]);
        return;
    }

    let len = name.chars().count();
    let out_of_bounds = len < policy.min_len as usize || len > policy.max_len as usize;
    let length = Rule::Length {
        min: policy.min_len,
        max: policy.max_len,
    };
    assert_eq!(full.contains(&length), out_of_bounds, "{full:?}");

    // The SQL text[] carries names only; parse_rule must restore the
    // parameters from the policy.
    for rule in &full {
        assert_eq!(policy.parse_rule(rule.as_str()), Ok(*rule));
    }

    let structural = check_structural(name, policy);
    assert_ordered(&structural);
    for rule in &structural {
        assert!(full.contains(rule), "structural {rule:?} not in {full:?}");
    }
    if name.is_ascii() {
        let d = Rule::DisallowedChar;
        assert_eq!(
            structural.contains(&d),
            full.contains(&d),
            "ASCII: structural {structural:?}, full {full:?}"
        );
    }
}

/// `normalize` produces a valid, NFC, idempotent storage form that keys
/// like its input, or a non-empty, ordered refusal without `not_nfc`.
pub fn normalize_properties(policy: &Policy, name: &str) {
    match normalize(name, policy) {
        Ok(stored) => {
            assert!(is_nfc(&stored), "{stored:?}");
            assert_eq!(check(&stored, policy), [], "{stored:?}");
            assert_eq!(normalize(&stored, policy).as_ref(), Ok(&stored));
            for (f, label) in KEYS {
                assert_eq!(f(name), f(&stored), "{label} of input vs storage form");
            }
        }
        Err(Rejected(rules)) => {
            assert!(!rules.is_empty());
            assert!(!rules.contains(&Rule::NotNfc), "{rules:?}");
            assert_ordered(&rules);
        }
    }
}

type KeyFn = fn(&str) -> String;

const KEYS: [(KeyFn, &str); 4] = [
    (key_ci, "key_ci"),
    (key_cs, "key_cs"),
    (key_visual_ci, "key_visual_ci"),
    (key_visual_cs, "key_visual_cs"),
];

/// Every key is invariant under the ingress the keys promise to absorb:
/// canonical equivalence, edge spaces and the RFC 8266 space mapping.
pub fn key_properties(name: &str) {
    let nfd: String = name.nfd().collect();
    let nfc: String = name.nfc().collect();
    let padded = format!(" \u{00A0}{name}\u{3000} ");
    let mapped = name.replace(' ', "\u{2009}");
    for (f, label) in KEYS {
        let key = f(name);
        assert_eq!(f(&nfd), key, "{label}: NFD");
        assert_eq!(f(&nfc), key, "{label}: NFC");
        assert_eq!(f(&padded), key, "{label}: edge spaces");
        assert_eq!(f(&mapped), key, "{label}: U+0020 as U+2009");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use restrictive_usernames::corpus::{CHECK, KEY_PAIRS, NORMALIZE, VISUAL_KEY_PAIRS};

    fn names() -> impl Iterator<Item = &'static str> {
        CHECK
            .iter()
            .map(|c| c.input)
            .chain(NORMALIZE.iter().map(|c| c.input))
            .chain(KEY_PAIRS.iter().flat_map(|p| [p.a, p.b]))
            .chain(VISUAL_KEY_PAIRS.iter().flat_map(|p| [p.a, p.b]))
    }

    #[test]
    fn encoding_round_trips() {
        for kind in KINDS {
            let input = encode(kind, "Jörg");
            let (policy, name) = decode(&input).unwrap();
            assert_eq!((policy, name), (Policy::for_kind(kind), "Jörg"));
        }
        let (policy, name) = decode(&[CUSTOM_POLICY | 1, 2, 9, 3, b'a']).unwrap();
        assert_eq!(policy.kind, Kind::Nickname);
        assert_eq!(
            (policy.min_len, policy.max_len, policy.max_mark_run),
            (2, 9, 3)
        );
        assert_eq!(name, "a");
        assert_eq!(decode(&[0, 0xFF]), None);
    }

    #[test]
    fn corpus_satisfies_the_properties() {
        for name in names() {
            for kind in KINDS {
                let policy = Policy::for_kind(kind);
                check_properties(&policy, name);
                normalize_properties(&policy, name);
            }
            key_properties(name);
        }
    }

    /// Writes the acceptance corpus as seed inputs into `corpus/<target>/`.
    /// Run before fuzzing: `cargo test -- --ignored write_seed_corpus`.
    #[test]
    #[ignore = "writes files; run on demand to seed the fuzzer"]
    fn write_seed_corpus() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        let names: Vec<&str> = names().collect();
        for target in ["check", "normalize", "keys"] {
            let dir = root.join(target);
            std::fs::create_dir_all(&dir).unwrap();
            for (i, name) in names.iter().enumerate() {
                if target == "keys" {
                    std::fs::write(dir.join(format!("seed-{i}")), name).unwrap();
                    continue;
                }
                for kind in KINDS {
                    let file = dir.join(format!("seed-{i}-{}", kind.as_str()));
                    std::fs::write(file, encode(kind, name)).unwrap();
                }
            }
        }
    }
}
