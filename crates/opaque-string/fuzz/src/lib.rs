//! Properties the fuzz targets assert.
//!
//! They live here rather than in the targets so that plain `cargo test`
//! (stable, no sanitizer) runs them over the shared corpus, and so that a
//! crash found by the fuzzer reproduces as an ordinary test. Both targets
//! take the whole input as the password, which must be UTF-8.

use opaque_string::{Prepared, Refused, enforce, prepare, refusal};
use unicode_normalization::is_nfc;

/// `prepare` is idempotent, returns NFC, maps every non-ASCII space, and is
/// empty exactly for empty input.
pub fn prepare_properties(password: &str) {
    let once = prepare(password);
    let twice = prepare(once.as_str());
    assert_eq!(once.as_str(), twice.as_str(), "not idempotent");
    assert!(is_nfc(once.as_str()), "not NFC");
    assert!(
        !once.as_str().chars().any(|c| !c.is_ascii()
            && matches!(
                c,
                '\u{00A0}' | '\u{1680}' | '\u{2000}'
                    ..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
            )),
        "a non-ASCII space survived"
    );
    assert_eq!(once.is_empty(), password.is_empty());
}

/// `enforce` agrees with `prepare`, is a fixed point on what it accepts,
/// reports the first refused code point and the count exactly, and never
/// shows the password in an error.
pub fn enforce_properties(password: &str) {
    let prepared = prepare(password);
    let result = enforce(password);
    assert_eq!(
        enforce(prepared.as_str()).as_ref().map(Prepared::as_str),
        result.as_ref().map(Prepared::as_str),
        "enforce(prepare(x)) != enforce(x)"
    );
    match &result {
        Ok(accepted) => {
            assert_eq!(accepted.as_str(), prepared.as_str());
            assert_eq!(
                enforce(accepted.as_str()).as_ref().map(Prepared::as_str),
                Ok(accepted.as_str())
            );
            assert!(accepted.as_str().chars().all(|c| refusal(c).is_none()));
            assert!(!accepted.is_empty());
        }
        Err(Refused::Empty) => assert!(prepared.is_empty()),
        Err(Refused::Disallowed { first, count }) => {
            let refused: Vec<(usize, char)> = prepared
                .as_str()
                .chars()
                .enumerate()
                .filter(|&(_, c)| refusal(c).is_some())
                .collect();
            assert_eq!(refused.first(), Some(&(first.index, first.code_point)));
            assert_eq!(refusal(first.code_point), Some(first.reason));
            assert_eq!(refused.len(), count.get());
        }
        Err(other) => panic!("unexpected refusal {other:?}"),
    }
    // The error text names code points as U+XXXX; it must not carry the
    // password itself.
    if let Err(e) = &result
        && password.chars().count() >= 8
    {
        assert!(!e.to_string().contains(password));
        assert!(!format!("{e:?}").contains(password));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const CORPUS: &str = include_str!("../../tests/corpus.json");

    fn inputs() -> Vec<String> {
        let corpus: Value = serde_json::from_str(CORPUS).unwrap();
        corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["input"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn corpus_satisfies_the_properties() {
        for input in inputs() {
            prepare_properties(&input);
            enforce_properties(&input);
        }
    }

    /// Writes the shared corpus as seed inputs into `corpus/<target>/`.
    /// Run before fuzzing: `cargo test -- --ignored write_seed_corpus`.
    #[test]
    #[ignore = "writes files; run on demand to seed the fuzzer"]
    fn write_seed_corpus() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        for target in ["prepare", "enforce"] {
            let dir = root.join(target);
            std::fs::create_dir_all(&dir).unwrap();
            for (i, input) in inputs().iter().enumerate() {
                std::fs::write(dir.join(format!("seed-{i}")), input).unwrap();
            }
        }
    }
}
