//! JavaScript bindings for `restrictive-usernames`, through wasm-bindgen.
//!
//! Two npm packages come from this crate (`build-npm.sh`):
//!
//! * `restrictive-usernames`: built without features. The vocabulary and
//!   [`check_structural`], no Unicode tables; small enough for a browser
//!   bundle, for instant feedback while a name is typed.
//! * `restrictive-usernames-full`: feature `check`. Adds the full policy
//!   ([`check`], [`is_valid`], [`normalize`]) and the confusable-skeleton
//!   keys, for a server or a client that must agree with the database.
//!
//! Kinds and rules cross as their serde names (`"username"`,
//! `"mixed_script"`); an unknown kind throws. Every function that takes a
//! kind also takes optional `minLen` / `maxLen` overrides of the kind's
//! default lengths (code points, inclusive), as
//! [`Policy::with_min_len`] / [`Policy::with_max_len`].

mod error;

use js_sys::{Array, Object, Reflect};
use restrictive_usernames::{Kind, Policy, Rule};
use wasm_bindgen::prelude::*;

pub use error::UnknownKind;

/// The TypeScript vocabulary. A plain const as well as the custom section
/// so a test can hold the `RuleName` union to [`Rule::ALL`].
#[cfg_attr(
    not(test),
    allow(dead_code, reason = "the custom section takes a copy of the string")
)]
const VOCABULARY: &str = r#"
/** Which profile a name is checked against. */
export type Kind = "username" | "nickname" | "label";

/** A rule name, in reporting order: the values of `ruleNames()`. */
export type RuleName =
  | "empty"
  | "length"
  | "first_not_letter"
  | "whitespace"
  | "edge_whitespace"
  | "control_char"
  | "disallowed_char"
  | "not_nfc"
  | "mixed_script"
  | "mark_run"
  | "no_base"
  | "not_taken";

/** A kind's default parameters: the values `minLen` / `maxLen` override. */
export interface PolicyDefaults {
  kind: Kind;
  minLen: number;
  maxLen: number;
  maxMarkRun: number;
}
"#;

#[wasm_bindgen(typescript_custom_section)]
const TS_VOCABULARY: &'static str = VOCABULARY;

#[cfg(feature = "check")]
#[wasm_bindgen(typescript_custom_section)]
const TS_NORMALIZED: &'static str = r#"
/** `normalize`: the storage form, or every rule it fails. */
export type Normalized =
  | { ok: true; value: string }
  | { ok: false; rules: RuleName[] };
"#;

/// The policy for `kind`, with the length overrides applied.
pub fn policy(
    kind: &str,
    min_len: Option<u32>,
    max_len: Option<u32>,
) -> Result<Policy, UnknownKind> {
    let kind = match kind {
        "username" => Kind::Username,
        "nickname" => Kind::Nickname,
        "label" => Kind::Label,
        other => return Err(other.into()),
    };
    let mut p = Policy::for_kind(kind);
    if let Some(n) = min_len {
        p = p.with_min_len(n);
    }
    if let Some(n) = max_len {
        p = p.with_max_len(n);
    }
    Ok(p)
}

/// Rule names as a JavaScript array of strings.
fn names(rules: &[Rule]) -> Array {
    rules
        .iter()
        .map(|r| JsValue::from_str(r.as_str()))
        .collect()
}

/// `target[key] = value`. Cannot fail on an ordinary object this module
/// just created, so the `Result` is dropped.
fn set(target: &Object, key: &str, value: &JsValue) {
    let _ = Reflect::set(target, &JsValue::from_str(key), value);
}

/// Version of the policy: the rules plus every Unicode table they read. Equal
/// to the SQL `restrictive_usernames.version()` of a database running the
/// same release; keys from a different version may not match the stored
/// ones.
#[wasm_bindgen(js_name = policyVersion)]
pub fn policy_version() -> String {
    restrictive_usernames::POLICY_VERSION.to_owned()
}

/// Every rule name, in reporting order.
#[wasm_bindgen(js_name = ruleNames, unchecked_return_type = "RuleName[]")]
pub fn rule_names() -> Array {
    names(Rule::ALL)
}

/// The default parameters of `kind`. Throws on an unknown kind.
#[wasm_bindgen(js_name = defaultPolicy, unchecked_return_type = "PolicyDefaults")]
pub fn default_policy(
    #[wasm_bindgen(unchecked_param_type = "Kind")] kind: &str,
) -> Result<Object, JsError> {
    let p = policy(kind, None, None)?;
    let o = Object::new();
    set(&o, "kind", &JsValue::from_str(p.kind.as_str()));
    set(&o, "minLen", &p.min_len.into());
    set(&o, "maxLen", &p.max_len.into());
    set(&o, "maxMarkRun", &p.max_mark_run.into());
    Ok(o)
}

/// The structural rules `input` fails, decidable without Unicode tables:
/// `empty` (reported alone), `length`, `first_not_letter`, `whitespace`,
/// `edge_whitespace`, `control_char`, and `disallowed_char` as far as ASCII
/// decides it. Always a subset of what the full `check` reports. An empty
/// array means nothing structural is wrong. Throws on an unknown kind.
#[wasm_bindgen(js_name = checkStructural, unchecked_return_type = "RuleName[]")]
pub fn check_structural(
    input: &str,
    #[wasm_bindgen(unchecked_param_type = "Kind")] kind: &str,
    #[wasm_bindgen(js_name = minLen)] min_len: Option<u32>,
    #[wasm_bindgen(js_name = maxLen)] max_len: Option<u32>,
) -> Result<Array, JsError> {
    let p = policy(kind, min_len, max_len)?;
    Ok(names(&restrictive_usernames::check_structural(input, &p)))
}

/// Every rule the storage form `input` fails, in reporting order. An empty
/// array means valid. Checks `input` as stored: run typed text through
/// `normalize` first. Throws on an unknown kind.
#[cfg(feature = "check")]
#[wasm_bindgen(unchecked_return_type = "RuleName[]")]
pub fn check(
    input: &str,
    #[wasm_bindgen(unchecked_param_type = "Kind")] kind: &str,
    #[wasm_bindgen(js_name = minLen)] min_len: Option<u32>,
    #[wasm_bindgen(js_name = maxLen)] max_len: Option<u32>,
) -> Result<Array, JsError> {
    let p = policy(kind, min_len, max_len)?;
    Ok(names(&restrictive_usernames::check(input, &p)))
}

/// `check(input, kind, minLen, maxLen).length === 0`. Throws on an unknown
/// kind.
#[cfg(feature = "check")]
#[wasm_bindgen(js_name = isValid)]
pub fn is_valid(
    input: &str,
    #[wasm_bindgen(unchecked_param_type = "Kind")] kind: &str,
    #[wasm_bindgen(js_name = minLen)] min_len: Option<u32>,
    #[wasm_bindgen(js_name = maxLen)] max_len: Option<u32>,
) -> Result<bool, JsError> {
    let p = policy(kind, min_len, max_len)?;
    Ok(restrictive_usernames::is_valid(input, &p))
}

/// Ingress: typed text to its storage form. Nicknames and labels map every
/// space to U+0020, collapse runs and trim (RFC 8266 §2.2); then NFC; then
/// `check`. `{ ok: true, value }` is the form to store; `{ ok: false, rules }`
/// lists every rule it fails (never `not_nfc`). Throws on an unknown kind.
#[cfg(feature = "check")]
#[wasm_bindgen(unchecked_return_type = "Normalized")]
pub fn normalize(
    input: &str,
    #[wasm_bindgen(unchecked_param_type = "Kind")] kind: &str,
    #[wasm_bindgen(js_name = minLen)] min_len: Option<u32>,
    #[wasm_bindgen(js_name = maxLen)] max_len: Option<u32>,
) -> Result<Object, JsError> {
    let p = policy(kind, min_len, max_len)?;
    let o = Object::new();
    match restrictive_usernames::normalize(input, &p) {
        Ok(value) => {
            set(&o, "ok", &JsValue::TRUE);
            set(&o, "value", &JsValue::from_str(&value));
        }
        Err(rejected) => {
            set(&o, "ok", &JsValue::FALSE);
            set(&o, "rules", &names(&rejected.0));
        }
    }
    Ok(o)
}

/// The case-insensitive confusable key: `ADMIN` / `admin`, `pаypal`
/// (Cyrillic а) / `paypal`, `Straße` / `Strasse` collide. Two names collide
/// when this key or `keyCs` is equal.
#[cfg(feature = "check")]
#[wasm_bindgen(js_name = keyCi)]
pub fn key_ci(input: &str) -> String {
    restrictive_usernames::key_ci(input)
}

/// The as-typed confusable key, skeleton before folding: catches `Iris` /
/// `lris` and `Bob` / `B0b`, which `keyCi` folds away. Use both keys.
#[cfg(feature = "check")]
#[wasm_bindgen(js_name = keyCs)]
pub fn key_cs(input: &str) -> String {
    restrictive_usernames::key_cs(input)
}

/// `keyCi` of the displayed form (UAX #9 order, first-strong direction):
/// `Dan אבי` / `אבי Dan` collide. For nicknames; pair with `keyVisualCs`.
#[cfg(feature = "check")]
#[wasm_bindgen(js_name = keyVisualCi)]
pub fn key_visual_ci(input: &str) -> String {
    restrictive_usernames::key_visual_ci(input)
}

/// `keyCs` of the displayed form: `Iris אבי` / `אבי lris` collide.
#[cfg(feature = "check")]
#[wasm_bindgen(js_name = keyVisualCs)]
pub fn key_visual_cs(input: &str) -> String {
    restrictive_usernames::key_visual_cs(input)
}

/// The Unicode version of every table the policy reads, one per crate.
#[cfg(feature = "check")]
#[wasm_bindgen(js_name = unicodeVersion)]
pub fn unicode_version() -> String {
    restrictive_usernames::UNICODE_VERSION.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_parse_by_serde_name() {
        for k in [Kind::Username, Kind::Nickname, Kind::Label] {
            assert_eq!(policy(k.as_str(), None, None), Ok(Policy::for_kind(k)));
        }
        assert_eq!(
            policy("Username", None, None),
            Err(UnknownKind("Username".into()))
        );
    }

    #[test]
    fn length_overrides_apply() {
        let p = policy("username", Some(5), Some(9)).unwrap();
        assert_eq!(p, Policy::username().with_min_len(5).with_max_len(9));
    }

    /// The `RuleName` union lists exactly `Rule::ALL`, in order.
    #[test]
    fn rule_name_union_matches_rule_all() {
        let union = VOCABULARY
            .split("export type RuleName =")
            .nth(1)
            .and_then(|s| s.split(';').next())
            .unwrap();
        let listed: Vec<&str> = union
            .split('|')
            .map(|s| s.trim().trim_matches('"'))
            .filter(|s| !s.is_empty())
            .collect();
        let all: Vec<&str> = Rule::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(listed, all);
    }
}
