//! Unicode username, nickname and label policy.
//!
//! Three kinds of name, one vocabulary of failures ([`Rule`]):
//!
//! * [`Kind::Username`]: the shape of the PRECIS IdentifierClass
//!   ([RFC 8264 §4.2], profiled by [RFC 8265]) restricted to the UTS #39
//!   identifier profile: letters, digits, marks and `_`, first code point a
//!   letter, one word.
//! * [`Kind::Nickname`]: the shape of the PRECIS Nickname profile
//!   ([RFC 8266]): the same identifier profile plus its punctuation, words
//!   joined by single U+0020.
//! * [`Kind::Label`]: free text for room, scene or avatar names: anything but
//!   invisibles, noncharacters, deprecated code points and controls.
//!
//! Usernames and nicknames are also held to the UTS #39 §5.2 *Highly
//! Restrictive* level, per word, and every name gets the mark-run and base
//! character rules that close Zalgo text.
//!
//! [`key_ci`] and [`key_cs`] compute UTS #39 §4 confusable-skeleton keys for
//! unique expression indexes: two names collide when either key does.
//! [`key_visual_ci`] and [`key_visual_cs`] are the same keys over the UAX #9
//! display order, for nicknames, whose words of opposite direction can be
//! stored in different orders yet display alike (`Dan אבי` / `אבי Dan`).
//!
//! The `check` feature (default) carries the Unicode tables. Without it the
//! crate is the vocabulary plus [`check_structural`], which needs only `core`
//! and is what a browser build takes for instant feedback; the server and the
//! database run the full [`check`].
//!
//! # Example
//!
//! ```
//! # #[cfg(feature = "check")] {
//! use restrictive_usernames::{Policy, Rule, check, key_ci, key_cs, normalize};
//!
//! let nick = Policy::nickname();
//!
//! // Ingress: map spaces, NFC, then every rule.
//! assert_eq!(normalize("Jean\u{3000}Luc", &nick).unwrap(), "Jean Luc");
//! // Storage form: every failed rule, in reporting order.
//! assert_eq!(check("\u{410}dmin", &nick), [Rule::MixedScript]); // Cyrillic А
//! assert_eq!(check("ローマ字Latin", &nick), []);
//!
//! // Uniqueness: two names collide when either key does.
//! assert_eq!(key_ci("ADMIN"), key_ci("admin"));
//! assert_eq!(key_cs("p\u{430}ypal"), key_cs("paypal")); // Cyrillic а
//! assert_ne!(key_cs("Jörg"), key_cs("Jorg"));
//! # }
//! ```
//!
//! Without the `check` feature, [`check_structural`] gives a browser the
//! rules it can decide without Unicode tables:
//!
//! ```
//! use restrictive_usernames::{Policy, Rule, check_structural};
//!
//! let user = Policy::username();
//! assert_eq!(check_structural("user-name", &user), [Rule::DisallowedChar]);
//! assert_eq!(
//!     check_structural("ab", &user),
//!     [Rule::Length { min: 3, max: 32 }]
//! );
//! ```
//!
//! [RFC 8264 §4.2]: https://www.rfc-editor.org/rfc/rfc8264#section-4.2
//! [RFC 8265]: https://www.rfc-editor.org/rfc/rfc8265
//! [RFC 8266]: https://www.rfc-editor.org/rfc/rfc8266

mod error;
mod kind;
mod policy;
mod rule;
mod structural;

#[cfg(feature = "check")]
mod check;
#[cfg(feature = "check")]
mod key;

#[cfg(feature = "corpus")]
pub mod corpus;

pub use error::{ParseRuleError, Rejected};
pub use kind::Kind;
pub use policy::Policy;
pub use rule::Rule;
pub use structural::check_structural;

#[cfg(feature = "check")]
pub use check::{check, is_valid, normalize};
#[cfg(feature = "check")]
pub use key::{key_ci, key_cs, key_visual_ci, key_visual_cs};

/// Version of the policy: the rules in this crate plus every Unicode table
/// they read.
///
/// Bumped on any rule change or any Unicode data change (see
/// [`UNICODE_VERSION`]), because either can change what [`check`] accepts and
/// what [`key_ci`] / [`key_cs`] return. A bump obliges every database to load
/// the new library, find and resolve rows the new policy rejects or makes
/// collide, and `REINDEX` every index built on the keys, before any consumer
/// compiled against the new crate starts (README, "Upgrades"). The SQL
/// function `restrictive_usernames.version()` returns this value.
///
/// History: "1" initial; "2" marks counted by General Category M* instead of
/// combining class, repeated-mark and leading-mark refusal, nicknames need a
/// letter or digit; "3" reserved prefixes removed from the policy and SQL
/// API, structural tier refuses ASCII outside the kind's profile; `check` and
/// key output unchanged for every name, so the `REINDEX` is a no-op; "4"
/// adds the display-order keys and the UAX #9 tables they read; `check`,
/// `key_ci` and `key_cs` unchanged; "5" `key_visual_ci` / `key_visual_cs`
/// apply UAX #9 rule L3 (marks stay after their base in right-to-left runs),
/// which changes them for right-to-left names with marks; `check`, `key_ci`
/// and `key_cs` unchanged.
pub const POLICY_VERSION: &str = "5";

/// The Unicode version of every table the policy reads, one per crate.
///
/// * `security`: UTS #39 Identifier_Type and confusables (`unicode-security`).
/// * `normalization`: NFC, NFD, NFKC and canonical combining classes
///   (`unicode-normalization`).
/// * `script`: Script_Extensions for the mixed-script rule (`unicode-script`).
/// * `ident`: XID_Start / XID_Continue (`unicode-ident`).
/// * `casefold`: full case folding, CaseFolding.txt status C+F (`caseless`).
/// * `bidi`: Bidi_Class for the display-order keys (`unicode-bidi`).
///
/// The six crates are pinned to exact versions, and a compile-time assertion
/// below holds each one's own version constant to the version named here,
/// so a build that links other tables (a `[patch]`, a forked crate) fails
/// to compile, in this crate and in every crate that depends on it, rather
/// than computing keys that disagree with the database's. A unit test
/// asserts this string against the same constants, so moving a table cannot
/// land without bumping this string and [`POLICY_VERSION`] together.
pub const UNICODE_VERSION: &str = "security 16.0.0; normalization 17.0.0; script 17.0.0; ident 18.0.0; casefold 16.0.0; bidi 16.0.0";

/// The tables the policy was written against. Keys computed with any other
/// tables would not match the ones stored in a database's unique indexes,
/// so linking them is a compile error, not a test failure.
#[cfg(feature = "check")]
const _: () = {
    const fn is(v: (u64, u64, u64), major: u64, minor: u64, patch: u64) -> bool {
        v.0 == major && v.1 == minor && v.2 == patch
    }
    const fn wide(v: (u8, u8, u8)) -> (u64, u64, u64) {
        (v.0 as u64, v.1 as u64, v.2 as u64)
    }
    assert!(
        is(unicode_security::UNICODE_VERSION, 16, 0, 0),
        "unicode-security tables moved: see UNICODE_VERSION"
    );
    assert!(
        is(wide(unicode_normalization::UNICODE_VERSION), 17, 0, 0),
        "unicode-normalization tables moved: see UNICODE_VERSION"
    );
    assert!(
        is(unicode_script::UNICODE_VERSION, 17, 0, 0),
        "unicode-script tables moved: see UNICODE_VERSION"
    );
    assert!(
        is(wide(unicode_ident::UNICODE_VERSION), 18, 0, 0),
        "unicode-ident tables moved: see UNICODE_VERSION"
    );
    assert!(
        is(caseless::UNICODE_VERSION, 16, 0, 0),
        "caseless tables moved: see UNICODE_VERSION"
    );
    assert!(
        is(unicode_bidi::UNICODE_VERSION, 16, 0, 0),
        "unicode-bidi tables moved: see UNICODE_VERSION"
    );
};

#[cfg(all(test, feature = "check"))]
mod version_tests {
    use super::{POLICY_VERSION, UNICODE_VERSION};

    /// Every released (POLICY_VERSION, UNICODE_VERSION) pair, oldest first.
    /// Append-only. A Unicode table move makes the linked versions differ
    /// from UNICODE_VERSION; fixing that makes the last entry stale; the new
    /// entry needs a policy version no earlier entry used. So a table move
    /// cannot land without a deliberate bump of both constants.
    const HISTORY: &[(&str, &str)] = &[
        (
            "1",
            "security 16.0.0; normalization 17.0.0; script 17.0.0; ident 18.0.0; casefold 16.0.0",
        ),
        (
            "2",
            "security 16.0.0; normalization 17.0.0; script 17.0.0; ident 18.0.0; casefold 16.0.0",
        ),
        (
            "3",
            "security 16.0.0; normalization 17.0.0; script 17.0.0; ident 18.0.0; casefold 16.0.0",
        ),
        (
            "4",
            "security 16.0.0; normalization 17.0.0; script 17.0.0; ident 18.0.0; casefold 16.0.0; bidi 16.0.0",
        ),
        (
            "5",
            "security 16.0.0; normalization 17.0.0; script 17.0.0; ident 18.0.0; casefold 16.0.0; bidi 16.0.0",
        ),
    ];

    fn v3<T: core::fmt::Display>(v: (T, T, T)) -> String {
        format!("{}.{}.{}", v.0, v.1, v.2)
    }

    #[test]
    fn policy_and_unicode_versions_move_together() {
        let linked = format!(
            "security {}; normalization {}; script {}; ident {}; casefold {}; bidi {}",
            v3(unicode_security::UNICODE_VERSION),
            v3(unicode_normalization::UNICODE_VERSION),
            v3(unicode_script::UNICODE_VERSION),
            v3(unicode_ident::UNICODE_VERSION),
            v3(caseless::UNICODE_VERSION),
            v3(unicode_bidi::UNICODE_VERSION),
        );
        assert_eq!(
            UNICODE_VERSION, linked,
            "a Unicode table moved: update UNICODE_VERSION, bump POLICY_VERSION, append to HISTORY"
        );
        assert_eq!(
            HISTORY.last().copied(),
            Some((POLICY_VERSION, UNICODE_VERSION)),
            "(POLICY_VERSION, UNICODE_VERSION) is not the last HISTORY entry"
        );
        for (i, (p, _)) in HISTORY.iter().enumerate() {
            assert!(
                HISTORY[..i].iter().all(|(q, _)| q != p),
                "policy version {p} reused in HISTORY"
            );
        }
    }
}
