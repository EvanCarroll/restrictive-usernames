//! The shared acceptance corpus (tests/corpus.json) against the compiled-in
//! NFC tables. The npm tests run the same file against the engine's NFC.
#![cfg(feature = "nfc-tables")]

use opaque_string::{Refused, enforce, prepare};
use serde_json::Value;

const CORPUS: &str = include_str!("corpus.json");

fn code_point(s: &str) -> char {
    let hex = s.strip_prefix("U+").expect("U+XXXX");
    char::from_u32(u32::from_str_radix(hex, 16).expect("hex")).expect("scalar value")
}

#[test]
fn corpus() {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    assert_eq!(corpus["unicode"], opaque_string::UNICODE_VERSION);
    assert_eq!(corpus["nfc_baseline"], opaque_string::NFC_BASELINE);
    let mut failures = Vec::new();
    for case in corpus["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let input = case["input"].as_str().unwrap();
        let prepared = prepare(input);
        if prepared.as_str() != case["prepare"].as_str().unwrap() {
            failures.push(format!("{name}: prepare gave {:?}", prepared.as_str()));
        }
        let want = &case["enforce"];
        let ok = match (enforce(input), want["ok"].as_str(), want["reason"].as_str()) {
            (Ok(p), Some(v), _) => p.as_str() == v,
            (Err(Refused::Empty), _, Some("empty")) => true,
            (Err(Refused::Disallowed { first, count }), _, Some(reason)) => {
                first.reason.as_str() == reason
                    && first.code_point == code_point(want["code_point"].as_str().unwrap())
                    && first.index as u64 == want["index"].as_u64().unwrap()
                    && count.get() as u64 == want["count"].as_u64().unwrap()
            }
            (got, ..) => {
                failures.push(format!("{name}: enforce gave {got:?}"));
                continue;
            }
        };
        if !ok {
            failures.push(format!("{name}: enforce gave {:?}", enforce(input)));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
