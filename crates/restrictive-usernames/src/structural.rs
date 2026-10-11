//! The structural tier: rules decidable with `core` alone.

use crate::kind::Kind;
use crate::policy::Policy;
use crate::rule::Rule;

/// The space characters mapped to U+0020 on Nickname and Label ingress: the
/// non-ASCII members of General Category Zs plus U+2028 and U+2029.
/// RFC 8266 §2.2 rule 1 ("Additional Mapping Rule").
#[cfg(feature = "check")]
pub(crate) const fn is_mapped_space(c: char) -> bool {
    matches!(
        c,
        '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

/// RFC 8266 §2.2 rule 1: map the Unicode spaces to U+0020, collapse runs of
/// U+0020 to one, remove leading and trailing U+0020. Idempotent on storage
/// forms.
#[cfg(feature = "check")]
pub(crate) fn prep(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_space = false;
    for c in input.chars() {
        let c = if is_mapped_space(c) { ' ' } else { c };
        if c == ' ' {
            pending_space = true;
        } else {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(c);
        }
    }
    out
}

/// An ordered, duplicate-free set of rules (ordered by [`Rule::ALL`]).
#[derive(Default)]
pub(crate) struct RuleSet([Option<Rule>; Rule::ALL.len()]);

impl RuleSet {
    pub(crate) fn insert(&mut self, rule: Rule) {
        self.0[rule.ordinal()] = Some(rule);
    }

    pub(crate) fn into_vec(self) -> Vec<Rule> {
        self.0.into_iter().flatten().collect()
    }
}

/// The std-only letter test: Alphabetic and not General Category N*.
/// Over the Username profile, [`crate::check`] tightens it with XID_Start,
/// which together are exactly General Category L*.
pub(crate) fn is_letter_core(c: char) -> bool {
    c.is_alphabetic() && !c.is_numeric()
}

/// Whether the ASCII code point `c` is in the profile of `kind`, ignoring
/// whitespace and controls (reported by their own rules). Over ASCII the
/// UTS #39 identifier profile is letters, digits and `_` (Recommended) plus
/// `'` `-` `.` `:` (Inclusion), which only the Nickname profile admits; the
/// Label profile admits all of ASCII. `ascii_structural_decides_disallowed_exactly`
/// holds this table to [`crate::check`] for every ASCII code point.
const fn ascii_allowed(kind: Kind, c: char) -> bool {
    match kind {
        Kind::Username => c.is_ascii_alphanumeric() || c == '_',
        Kind::Nickname => c.is_ascii_alphanumeric() || matches!(c, '_' | '\'' | '-' | '.' | ':'),
        Kind::Label => true,
    }
}

/// [`Rule::DisallowedChar`] as far as ASCII decides it, without tables: an
/// ASCII code point outside the kind's profile, or a nickname that is all
/// ASCII with no letter or digit. Not part of [`structural_into`]:
/// [`crate::check`] decides the same rule from the tables, so keeping this
/// out of it leaves the full check independent of the table above.
fn ascii_profile_into(input: &str, kind: Kind, out: &mut RuleSet) {
    let outside = input
        .chars()
        .filter(|c| c.is_ascii() && !c.is_whitespace() && !c.is_control())
        .any(|c| !ascii_allowed(kind, c));
    let no_letter_or_digit = kind == Kind::Nickname
        && input.is_ascii()
        && !input.chars().any(|c| c.is_ascii_alphanumeric());
    if outside || no_letter_or_digit {
        out.insert(Rule::DisallowedChar);
    }
}

pub(crate) fn structural_into(input: &str, policy: &Policy, out: &mut RuleSet) -> bool {
    if input.trim().is_empty() {
        out.insert(Rule::Empty);
        return false;
    }

    // A count past u32::MAX is past any max_len, so saturating is exact here.
    let len = u32::try_from(input.chars().count()).unwrap_or(u32::MAX);
    if len < policy.min_len || len > policy.max_len {
        out.insert(Rule::Length {
            min: policy.min_len,
            max: policy.max_len,
        });
    }

    match policy.kind {
        Kind::Username => {
            if input.chars().next().is_some_and(|c| !is_letter_core(c)) {
                out.insert(Rule::FirstNotLetter);
            }
            if input.chars().any(char::is_whitespace) {
                out.insert(Rule::Whitespace);
            }
        }
        Kind::Nickname | Kind::Label => {
            if input.chars().any(|c| c.is_whitespace() && c != ' ') || input.contains("  ") {
                out.insert(Rule::Whitespace);
            }
            if input.starts_with(' ') || input.ends_with(' ') {
                out.insert(Rule::EdgeWhitespace);
            }
        }
    }

    if input.chars().any(char::is_control) {
        out.insert(Rule::ControlChar);
    }
    true
}

/// The structural rules, decidable without Unicode tables: [`Rule::Empty`],
/// [`Rule::Length`], [`Rule::FirstNotLetter`] (Alphabetic and not N*),
/// [`Rule::Whitespace`] (White_Space), [`Rule::EdgeWhitespace`],
/// [`Rule::ControlChar`] (General Category Cc), and [`Rule::DisallowedChar`]
/// as far as ASCII decides it: for a username an ASCII code point other than
/// letters, digits and `_`; for a nickname the same but also admitting
/// `'` `-` `.` `:`, and an all-ASCII nickname with no letter or digit. Labels
/// get no [`Rule::DisallowedChar`] here.
///
/// These are the RFC 8264 §5.2 / RFC 8266 §2 shape rules that `core::char`
/// can answer. The result is always a subset of [`crate::check`] on the same
/// input, so a browser can show it instantly and let the server add the rest;
/// over ASCII, [`Rule::DisallowedChar`] is decided exactly as `check` decides
/// it. [`Rule::Empty`] is reported alone.
pub fn check_structural(input: &str, policy: &Policy) -> Vec<Rule> {
    let mut out = RuleSet::default();
    if structural_into(input, policy, &mut out) {
        ascii_profile_into(input, policy.kind, &mut out);
    }
    out.into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "check")]
    #[test]
    fn prep_maps_collapses_trims() {
        assert_eq!(prep("\u{3000} a\u{00A0}\u{2009} b  "), "a b");
        assert_eq!(prep("a b"), "a b");
        assert_eq!(prep("   "), "");
        assert_eq!(prep("a\tb"), "a\tb");
    }

    #[test]
    fn structural_rules() {
        let n = Policy::nickname();
        assert_eq!(check_structural("  ", &n), [Rule::Empty]);
        assert_eq!(check_structural("Bob", &n), []);
        assert_eq!(check_structural(" id:x", &n), [Rule::EdgeWhitespace]);
        assert_eq!(check_structural("a  b", &n), [Rule::Whitespace]);
        assert_eq!(
            check_structural("a\tb", &n),
            [Rule::Whitespace, Rule::ControlChar]
        );
        let u = Policy::username();
        assert_eq!(
            check_structural("9 x", &u),
            [Rule::FirstNotLetter, Rule::Whitespace]
        );
        for bad in ["user-name", "a.b", "id:x", "ab!"] {
            assert_eq!(check_structural(bad, &u), [Rule::DisallowedChar], "{bad}");
        }
        assert_eq!(check_structural("Jörg", &u), []);
        for bad in ["#x", "...", "'", "a!b"] {
            assert_eq!(check_structural(bad, &n), [Rule::DisallowedChar], "{bad}");
        }
        for ok in ["O'Brien", "Jean-Luc", "St. John", "a:b", "id:x", "\u{B7}"] {
            assert_eq!(check_structural(ok, &n), [], "{ok}");
        }
        let l = Policy::label();
        assert_eq!(check_structural("#x!", &l), []);
    }
}
