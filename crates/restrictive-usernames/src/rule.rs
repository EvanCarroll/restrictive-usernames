//! The vocabulary of failures.

use core::fmt;
use core::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::ParseRuleError;

/// One reason a name is refused.
///
/// The serde name (`snake_case`) is also [`Rule::as_str`], which is what the
/// SQL functions return in their `text[]`. Every checker reports rules in
/// [`Rule::ALL`] order without duplicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// The input is empty after trimming White_Space. Reported alone.
    /// (RFC 8266 §2.3: a nickname must not be empty after the rules.)
    Empty,
    /// The code-point count is outside `[min, max]` (RFC 8264 §5.2.6 leaves
    /// length to the application).
    Length {
        /// Inclusive minimum.
        min: u32,
        /// Inclusive maximum.
        max: u32,
    },
    /// Username: the first code point is not a letter (General Category L*).
    FirstNotLetter,
    /// Username: any White_Space code point (RFC 8264 §9.14, the
    /// IdentifierClass disallows spaces). Nickname and Label storage form: a
    /// space other than U+0020, or a run of U+0020 (RFC 8266 §2.2 maps both
    /// away on ingress).
    Whitespace,
    /// Storage form: leading or trailing U+0020 (RFC 8266 §2.2 rule 1 trims
    /// them on ingress).
    EdgeWhitespace,
    /// A General Category Cc code point (RFC 8264 §9.12, Controls).
    ControlChar,
    /// A code point outside the kind's profile: UTS #39 §3.1 Identifier_Type
    /// not allowed (symbols, emoji, Not_NFKC, Default_Ignorable, Exclusion,
    /// ...), or for a username anything but letters, digits, marks and `_`.
    /// Also a nickname with no letter or digit at all (`.`, `...`, `_`).
    DisallowedChar,
    /// Storage form only: the input is not in NFC (UAX #15; RFC 8264 §7
    /// normalization rule). [`crate::normalize`] never returns it.
    NotNfc,
    /// A word fails UTS #39 §5.2 Highly Restrictive: it is neither single
    /// script nor Latin with Japanese, Han with Bopomofo, or Korean.
    MixedScript,
    /// More than `max` consecutive marks, or the same mark twice in a row
    /// (General Category M*, NFD form; UTS #39 §5.4). A run of exactly `max`
    /// distinct marks is allowed.
    MarkRun {
        /// The policy's limit.
        max: u32,
    },
    /// A word starts with a mark or has only marks (General Category M*, NFD
    /// form): every mark must follow the code point it combines with.
    NoBase,
    /// The name is held by someone else. Never produced by this crate: an
    /// application-level rule kept in the vocabulary so a client checklist
    /// and the server share one enum.
    NotTaken,
}

impl Rule {
    /// Every rule, in reporting order. [`Rule::Length`] and
    /// [`Rule::MarkRun`] appear with zeroed parameters.
    pub const ALL: &'static [Rule] = &[
        Rule::Empty,
        Rule::Length { min: 0, max: 0 },
        Rule::FirstNotLetter,
        Rule::Whitespace,
        Rule::EdgeWhitespace,
        Rule::ControlChar,
        Rule::DisallowedChar,
        Rule::NotNfc,
        Rule::MixedScript,
        Rule::MarkRun { max: 0 },
        Rule::NoBase,
        Rule::NotTaken,
    ];

    /// The serde name, e.g. `"mixed_script"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Rule::Empty => "empty",
            Rule::Length { .. } => "length",
            Rule::FirstNotLetter => "first_not_letter",
            Rule::Whitespace => "whitespace",
            Rule::EdgeWhitespace => "edge_whitespace",
            Rule::ControlChar => "control_char",
            Rule::DisallowedChar => "disallowed_char",
            Rule::NotNfc => "not_nfc",
            Rule::MixedScript => "mixed_script",
            Rule::MarkRun { .. } => "mark_run",
            Rule::NoBase => "no_base",
            Rule::NotTaken => "not_taken",
        }
    }

    /// Whether a synchronous checker can decide the rule from the name
    /// alone: every rule but [`Rule::NotTaken`].
    pub const fn is_sync(self) -> bool {
        !matches!(self, Rule::NotTaken)
    }

    /// Position in [`Rule::ALL`].
    pub(crate) const fn ordinal(self) -> usize {
        match self {
            Rule::Empty => 0,
            Rule::Length { .. } => 1,
            Rule::FirstNotLetter => 2,
            Rule::Whitespace => 3,
            Rule::EdgeWhitespace => 4,
            Rule::ControlChar => 5,
            Rule::DisallowedChar => 6,
            Rule::NotNfc => 7,
            Rule::MixedScript => 8,
            Rule::MarkRun { .. } => 9,
            Rule::NoBase => 10,
            Rule::NotTaken => 11,
        }
    }
}

/// Inverse of [`Rule::as_str`]. [`Rule::Length`] and [`Rule::MarkRun`] come
/// back with zeroed parameters; use [`crate::Policy::parse_rule`] to fill them
/// from the policy that produced the name.
impl FromStr for Rule {
    type Err = ParseRuleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Rule::ALL
            .iter()
            .copied()
            .find(|r| r.as_str() == s)
            .ok_or_else(|| ParseRuleError(s.to_owned()))
    }
}

/// Short English, for consumers without localization.
impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Rule::Empty => f.write_str("must not be empty"),
            Rule::Length { min, max } => {
                write!(f, "must be {min} to {max} characters long")
            }
            Rule::FirstNotLetter => f.write_str("must start with a letter"),
            Rule::Whitespace => f.write_str("contains disallowed spacing"),
            Rule::EdgeWhitespace => f.write_str("must not start or end with a space"),
            Rule::ControlChar => f.write_str("contains a control character"),
            Rule::DisallowedChar => f.write_str("contains a character that is not allowed"),
            Rule::NotNfc => f.write_str("is not in Unicode normalization form C"),
            Rule::MixedScript => f.write_str("mixes writing systems within a word"),
            Rule::MarkRun { max } => {
                write!(
                    f,
                    "has more than {max} combining marks in a row, or one repeated"
                )
            }
            Rule::NoBase => f.write_str("has a combining mark without a letter before it"),
            Rule::NotTaken => f.write_str("is already taken"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_is_the_serde_name_and_round_trips() {
        for (i, rule) in Rule::ALL.iter().copied().enumerate() {
            assert_eq!(rule.ordinal(), i);
            let json = serde_json::to_value(rule).unwrap();
            let serde_name = match &json {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Object(m) => m.keys().next().unwrap().clone(),
                other => panic!("unexpected serde shape {other}"),
            };
            assert_eq!(serde_name, rule.as_str());
            assert_eq!(rule.as_str().parse::<Rule>().unwrap(), rule);
            let back: Rule = serde_json::from_value(json).unwrap();
            assert_eq!(back, rule);
        }
        assert!("nope".parse::<Rule>().is_err());
    }

    #[test]
    fn is_sync_excludes_only_not_taken() {
        let not_sync: Vec<_> = Rule::ALL.iter().filter(|r| !r.is_sync()).collect();
        assert_eq!(not_sync, [&Rule::NotTaken]);
    }

    #[test]
    fn policy_parse_rule_fills_parameters() {
        let p = crate::Policy::username();
        assert_eq!(
            p.parse_rule("length").unwrap(),
            Rule::Length { min: 3, max: 32 }
        );
        assert_eq!(p.parse_rule("mark_run").unwrap(), Rule::MarkRun { max: 4 });
        assert_eq!(p.parse_rule("no_base").unwrap(), Rule::NoBase);
    }
}
