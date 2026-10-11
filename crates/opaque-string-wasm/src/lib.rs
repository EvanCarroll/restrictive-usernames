//! JavaScript bindings for `opaque-string`, through wasm-bindgen, published
//! to npm as `opaque-string` (`build-npm.sh`).
//!
//! NFC comes from the engine's `String.prototype.normalize`, so the module
//! carries only the refusal table. `prepare` throws and `enforce` refuses
//! on an engine older than the Unicode baseline; `engineSupported()` says
//! which in advance.
//!
//! Inputs are taken as JavaScript strings, not converted by wasm-bindgen:
//! an unpaired surrogate would otherwise become U+FFFD without a word.

mod error;

use js_sys::{Array, JsString, Object, Reflect};
use opaque_string::{Reason, Refused};
use wasm_bindgen::prelude::*;

pub use error::Error;

/// The TypeScript vocabulary. A plain const as well as the custom section
/// so a test can hold the `ReasonName` union to [`Reason::ALL`].
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "the custom section takes a copy of the string")
)]
const VOCABULARY: &str = r#"
/** Why `enforce` refuses a code point: the values of `reasonNames()`. */
export type ReasonName =
  | "unassigned"
  | "control"
  | "ignorable"
  | "old_hangul_jamo"
  | "join_control"
  | "exception"
  | "disallowed"
  | "unstable_normalization";

/** `enforce`: the prepared password, or why it is refused. */
export type Enforced =
  | { ok: true; value: string }
  | { ok: false; reason: "empty" | "lone_surrogate" }
  | { ok: false; reason: ReasonName; codePoint: number; index: number; count: number };
"#;

#[wasm_bindgen(typescript_custom_section)]
const TS_VOCABULARY: &'static str = VOCABULARY;

/// `target[key] = value`. Cannot fail on an ordinary object this module
/// just created, so the `Result` is dropped.
fn set(target: &Object, key: &str, value: &JsValue) {
    let _ = Reflect::set(target, &JsValue::from_str(key), value);
}

fn text(input: &JsString) -> Result<String, Error> {
    if input.is_valid_utf16() {
        Ok(String::from(input))
    } else {
        Err(Error::LoneSurrogate)
    }
}

fn refused(reason: &str) -> Object {
    let o = Object::new();
    set(&o, "ok", &JsValue::FALSE);
    set(&o, "reason", &JsValue::from_str(reason));
    o
}

/// Login: the password's prepared form (non-ASCII spaces to U+0020, then
/// NFC). Refuses nothing. Throws on an unpaired surrogate or an engine
/// older than the baseline.
#[wasm_bindgen]
pub fn prepare(input: &JsString) -> Result<String, JsError> {
    let s = text(input).map_err(|e| JsError::new(e.message()))?;
    if !opaque_string::engine_supported() {
        let e = Error::Engine(opaque_string::UnsupportedEngine);
        return Err(JsError::new(e.message()));
    }
    Ok(opaque_string::prepare(&s).as_str().to_owned())
}

/// Registration and password change: `{ ok: true, value }` with the
/// prepared form, or `{ ok: false, reason, ... }`: `"empty"`,
/// `"lone_surrogate"`, or a `ReasonName` with the first refused code point,
/// its index in the prepared form (code points) and how many are refused.
/// Throws on an engine older than the baseline.
#[wasm_bindgen(unchecked_return_type = "Enforced")]
pub fn enforce(input: &JsString) -> Result<Object, JsError> {
    let Ok(s) = text(input) else {
        return Ok(refused("lone_surrogate"));
    };
    match opaque_string::enforce(&s) {
        Ok(prepared) => {
            let o = Object::new();
            set(&o, "ok", &JsValue::TRUE);
            set(&o, "value", &JsValue::from_str(prepared.as_str()));
            Ok(o)
        }
        Err(Refused::Empty) => Ok(refused("empty")),
        Err(Refused::Disallowed { first, count }) => {
            let o = refused(first.reason.as_str());
            set(&o, "codePoint", &u32::from(first.code_point).into());
            set(&o, "index", &(first.index as u32).into());
            set(&o, "count", &(count.get() as u32).into());
            Ok(o)
        }
        Err(Refused::UnsupportedEngine(e)) => Err(JsError::new(Error::Engine(e).message())),
        Err(_) => Err(JsError::new("unexpected refusal")),
    }
}

/// Whether this engine's `String.prototype.normalize` knows the Unicode
/// baseline (`nfcBaseline()`); if not, `prepare` and `enforce` throw.
#[wasm_bindgen(js_name = engineSupported)]
pub fn engine_supported() -> bool {
    opaque_string::engine_supported()
}

/// The Unicode version of the refusal table.
#[wasm_bindgen(js_name = unicodeVersion)]
pub fn unicode_version() -> String {
    opaque_string::UNICODE_VERSION.to_owned()
}

/// The Unicode version the engine's normalization must know.
#[wasm_bindgen(js_name = nfcBaseline)]
pub fn nfc_baseline() -> String {
    opaque_string::NFC_BASELINE.to_owned()
}

/// Every reason name, in code order.
#[wasm_bindgen(js_name = reasonNames, unchecked_return_type = "ReasonName[]")]
pub fn reason_names() -> Array {
    Reason::ALL
        .iter()
        .map(|r| JsValue::from_str(r.as_str()))
        .collect()
}

/// The version of this package (equal to the `opaque-string` crate's).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `ReasonName` union lists exactly `Reason::ALL`, in order.
    #[test]
    fn reason_name_union_matches_reason_all() {
        let union = VOCABULARY
            .split("export type ReasonName =")
            .nth(1)
            .and_then(|s| s.split(';').next())
            .unwrap();
        let listed: Vec<&str> = union
            .split('|')
            .map(|s| s.trim().trim_matches('"'))
            .filter(|s| !s.is_empty())
            .collect();
        let all: Vec<&str> = Reason::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(listed, all);
    }
}
