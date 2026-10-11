//! Properties over every Unicode scalar value, with the compiled-in tables.
#![cfg(feature = "nfc-tables")]

use opaque_string::{enforce, prepare, refusal};
use unicode_normalization::is_nfc;

/// The contexts each code point is prepared in: alone, after a base, and
/// between marks of different classes, where NFC reorders and composes.
fn contexts(c: char) -> [String; 4] {
    [
        c.to_string(),
        format!("a{c}"),
        format!("a{c}\u{0301}"),
        format!("{c}\u{0323}\u{0300}"),
    ]
}

#[test]
fn prepare_is_idempotent_and_nfc() {
    let mut failures = Vec::new();
    for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
        for s in contexts(c) {
            let once = prepare(&s);
            let twice = prepare(once.as_str());
            if once.as_str() != twice.as_str() || !is_nfc(once.as_str()) {
                failures.push(format!("U+{:04X} in {s:?}", u32::from(c)));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// An accepted password is a fixed point: enforcing what enforce returned
/// gives it back, and so does prepare.
#[test]
fn accepted_is_a_fixed_point() {
    for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
        for s in contexts(c) {
            if let Ok(p) = enforce(&s) {
                assert_eq!(enforce(p.as_str()).unwrap().as_str(), p.as_str());
                assert_eq!(prepare(p.as_str()).as_str(), p.as_str());
            }
        }
    }
}

/// NFC never turns an allowed code point into a refused one, so whether a
/// code point is allowed does not depend on whether it is typed composed.
#[test]
fn nfc_of_an_allowed_code_point_is_allowed() {
    let mut failures = Vec::new();
    for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
        if refusal(c).is_none()
            && prepare(&c.to_string())
                .as_str()
                .chars()
                .any(|d| refusal(d).is_some())
        {
            failures.push(format!("U+{:04X}", u32::from(c)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join(" "));
}

/// A tripwire: FNV-1a over the prepared form of every allowed code point,
/// alone and after "a". It changes only when the NFC tables or the refusal
/// table change, which is a minor release (CHANGELOG.md); update it then.
#[test]
fn golden_digest() {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
        if refusal(c).is_some() {
            continue;
        }
        for s in [c.to_string(), format!("a{c}")] {
            for b in prepare(&s).as_bytes().iter().chain([&0xFF]) {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0100_0000_01B3);
            }
        }
    }
    assert_eq!(h, 0x847d_5305_38b8_417f, "digest {h:#018x}");
}
