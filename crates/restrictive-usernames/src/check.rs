//! The full policy: identifier profile, script mixing, marks, NFC.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use unicode_script::Script;
use unicode_security::GeneralSecurityProfile;
use unicode_security::general_security_profile::IdentifierType;
use unicode_security::mixed_script::AugmentedScriptSet;

use crate::error::Rejected;
use crate::kind::Kind;
use crate::policy::Policy;
use crate::rule::Rule;
use crate::structural::{RuleSet, is_letter_core, prep, structural_into};

/// UTS #39 §3.1 identifier profile, Recommended + Inclusion + Limited_Use.
///
/// `unicode-security` reports a single Identifier_Type per code point, but
/// since Unicode 15.1 the property is multi-valued: the symbols and
/// punctuation of Limited_Use scripts (e.g. Javanese `꧁꧂`) are
/// `Limited_Use Not_XID` and come back as plain `Limited_Use`. Requiring
/// XID_Continue (UAX #31) on everything but Inclusion restores the Not_XID
/// half. Measured over all of Unicode: the Username form below is then
/// exactly General Category L*, M*, N* plus `_`.
fn in_identifier_profile(c: char, allow_inclusion: bool) -> bool {
    match c.identifier_type() {
        Some(IdentifierType::Inclusion) => allow_inclusion,
        Some(IdentifierType::Recommended | IdentifierType::Limited_Use) => {
            unicode_ident::is_xid_continue(c)
        }
        _ => false,
    }
}

/// Whether `c` is allowed in a name of `kind`, ignoring whitespace and
/// controls (reported by their own rules).
fn allowed(kind: Kind, c: char) -> bool {
    match kind {
        // Letters, digits, marks and `_`: the identifier profile without its
        // Inclusion punctuation.
        Kind::Username => in_identifier_profile(c, false),
        Kind::Nickname => in_identifier_profile(c, true),
        // Unassigned code points have no entry (Not_Character by default).
        Kind::Label => !matches!(
            c.identifier_type(),
            None | Some(
                IdentifierType::Not_Character
                    | IdentifierType::Default_Ignorable
                    | IdentifierType::Deprecated
            )
        ),
    }
}

/// UTS #39 §5.2 Highly Restrictive, minus its identifier-profile clause (the
/// profile is [`allowed`]'s job, and `unicode-security`'s
/// `detect_restriction_level` would refuse every Limited_Use code point).
///
/// The word passes when its resolved script set (§5.1, Common and Inherited
/// transparent, augmented with Jpan/Hanb/Kore) is non-empty, or when the
/// resolved set of its non-Latin code points still covers Japanese, Han with
/// Bopomofo, or Korean.
fn highly_restrictive(word: &str) -> bool {
    let mut all = AugmentedScriptSet::default();
    let mut non_latin = AugmentedScriptSet::default();
    for c in word.chars() {
        let set = AugmentedScriptSet::for_char(c);
        all.intersect_with(set);
        if !set.base.contains_script(Script::Latin) {
            non_latin.intersect_with(set);
        }
    }
    !all.is_empty() || non_latin.jpan || non_latin.hanb || non_latin.kore
}

/// UTS #39 §5.4 on the NFD form: more than `max` consecutive marks, or the
/// same mark twice in a row. "Mark" is General Category M* (Mn, Mc, Me), a
/// superset of the nonspacing marks §5.4 names, so spacing vowel signs with
/// canonical combining class 0 (Devanagari U+0941, Thai U+0E34, Khmer U+17B7)
/// count too. A run of exactly `max` distinct marks is allowed.
fn marks_violate_run(input: &str, max: u32) -> bool {
    let mut run = 0u32;
    let mut prev: Option<char> = None;
    for c in input.nfd() {
        if is_combining_mark(c) {
            run += 1;
            if run > max || prev == Some(c) {
                return true;
            }
            prev = Some(c);
        } else {
            run = 0;
            prev = None;
        }
    }
    false
}

/// A word lacks a base when its NFD form starts with a mark (a mark must
/// follow the code point it combines with) or consists only of marks.
fn lacks_base(word: &str) -> bool {
    word.nfd().next().is_none_or(is_combining_mark)
}

/// General Category L* or N*: over the identifier profile, the Username
/// profile minus marks and `_` (see [`in_identifier_profile`]).
fn is_letter_or_digit(c: char) -> bool {
    in_identifier_profile(c, false) && c != '_' && !is_combining_mark(c)
}

fn words(input: &str) -> impl Iterator<Item = &str> {
    input.split(char::is_whitespace).filter(|w| !w.is_empty())
}

/// The storage-form check: every rule of the policy, in [`Rule::ALL`] order.
/// An empty result means valid.
///
/// A superset of [`crate::check_structural`], adding
/// * [`Rule::DisallowedChar`]: the kind's profile (UTS #39 §3.1
///   Identifier_Type; see [`Kind`]), from the tables over every code point
///   (the structural tier decides only ASCII, and agrees with this there),
/// * [`Rule::NotNfc`]: UAX #15 NFC (RFC 8264 §7, normalization rule),
/// * [`Rule::MixedScript`]: UTS #39 §5.2 Highly Restrictive per word
///   (Username and Nickname),
/// * [`Rule::MarkRun`] and [`Rule::NoBase`]: UTS #39 §5.4 mark limits on
///   the NFD form (General Category M*): more than `max_mark_run` marks in a
///   row or the same mark twice in a row is `MarkRun`; a word that starts
///   with a mark or has only marks is `NoBase`,
/// * for nicknames, [`Rule::DisallowedChar`] also when no code point is a
///   letter or digit (General Category L* or N*): `.`, `...`, `_` alone would
///   be an invisible nameplate,
/// * for usernames, [`Rule::FirstNotLetter`] tightened from Alphabetic to
///   General Category L* (Alphabetic ∧ XID_Start ∧ ¬N*).
///
/// Never short-circuits: every failed rule is reported, except that
/// [`Rule::Empty`] is reported alone. Input is checked as stored; run
/// ingress text through [`normalize`] first.
pub fn check(input: &str, policy: &Policy) -> Vec<Rule> {
    let mut out = RuleSet::default();
    if !structural_into(input, policy, &mut out) {
        return out.into_vec();
    }
    let kind = policy.kind;

    if kind == Kind::Username
        && input
            .chars()
            .next()
            .is_some_and(|c| !(is_letter_core(c) && unicode_ident::is_xid_start(c)))
    {
        out.insert(Rule::FirstNotLetter);
    }

    if !unicode_normalization::is_nfc(input) {
        out.insert(Rule::NotNfc);
    }

    if input
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .any(|c| !allowed(kind, c))
    {
        out.insert(Rule::DisallowedChar);
    }

    if kind != Kind::Label && !words(input).all(highly_restrictive) {
        out.insert(Rule::MixedScript);
    }

    if marks_violate_run(input, policy.max_mark_run) {
        out.insert(Rule::MarkRun {
            max: policy.max_mark_run,
        });
    }

    if words(input).any(lacks_base) {
        out.insert(Rule::NoBase);
    }

    if kind == Kind::Nickname && !input.chars().any(is_letter_or_digit) {
        out.insert(Rule::DisallowedChar);
    }

    out.into_vec()
}

/// `check(input, policy).is_empty()`.
pub fn is_valid(input: &str, policy: &Policy) -> bool {
    check(input, policy).is_empty()
}

/// Ingress: turn typed text into its storage form, or say why it cannot be
/// stored.
///
/// For [`Kind::Nickname`] and [`Kind::Label`], RFC 8266 §2.2 rule 1 first:
/// U+00A0, U+1680, U+2000..=U+200A, U+2028, U+2029, U+202F, U+205F and
/// U+3000 become U+0020, runs collapse, ends are trimmed. Usernames are not
/// mapped (RFC 8265 §3.3: the IdentifierClass has no spaces to map). Then
/// NFC (RFC 8264 §7), then [`check`]. Never reports [`Rule::NotNfc`].
/// Case is preserved.
pub fn normalize(input: &str, policy: &Policy) -> Result<String, Rejected> {
    let mapped = match policy.kind {
        Kind::Username => input.to_owned(),
        Kind::Nickname | Kind::Label => prep(input),
    };
    let stored: String = mapped.nfc().collect();
    let rules = check(&stored, policy);
    if rules.is_empty() {
        Ok(stored)
    } else {
        debug_assert!(!rules.contains(&Rule::NotNfc));
        Err(Rejected(rules))
    }
}
