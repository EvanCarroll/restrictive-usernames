//! RFC 8265 §4.2 OpaqueString, the PRECIS profile for passwords.
//!
//! The same password typed on two devices can arrive as different text:
//! `é` precomposed on one keyboard and as `e` + U+0301 on another, or a
//! no-break space pasted from a password manager where a space was typed.
//! A password hash or a PAKE sees different bytes and the login fails.
//! OpaqueString fixes that and nothing else: it maps non-ASCII spaces to
//! U+0020 and applies NFC, and keeps case, width and everything else, so
//! the password keeps its entropy.
//!
//! This crate does no authentication. It is the step before it: feed
//! [`Prepared::as_bytes`] to an OPAQUE client (RFC 9807, e.g. `opaque-ke`)
//! or to a password hash. (The name is the RFC's; it is unrelated to the
//! OPAQUE protocol.)
//!
//! * [`enforce`] at registration and password change: prepares the password
//!   and refuses code points outside the profile (controls, invisibles,
//!   unassigned code points, ...).
//! * [`prepare`] at login: the same preparation, refusing nothing, so a
//!   password accepted once keeps working even if a later release refuses
//!   more.
//!
//! ```
//! # #[cfg(feature = "nfc-tables")] {
//! let login = opaque_string::prepare("correct horse\u{00A0}battery");
//! assert_eq!(login.as_str(), "correct horse battery");
//!
//! let registered = opaque_string::enforce("Cafe\u{0301} 2024").unwrap();
//! assert_eq!(registered.as_str(), "Caf\u{E9} 2024");
//!
//! let refused = opaque_string::enforce("pass\u{200B}word").unwrap_err();
//! assert_eq!(
//!     refused.to_string(),
//!     "password contains 1 disallowed code point(s); the first is U+200B (invisible or noncharacter)"
//! );
//! # }
//! ```
//!
//! # Deviations from RFC 8265
//!
//! * ZWJ and ZWNJ are refused everywhere (the RFC allows them in context).
//! * The RFC 5892 CONTEXTO code points (U+00B7, U+0375, U+05F3, U+05F4,
//!   U+30FB, the Arabic-Indic digits) are allowed anywhere: their context
//!   rules prevent look-alike *displayed* strings, and a password is never
//!   displayed.
//! * The class is checked on the prepared password, not on the raw input,
//!   so a modern Hangul syllable typed as conjoining jamo is accepted.
//! * Code points newer than Unicode [`NFC_BASELINE`] that NFC treats
//!   differently on an older engine are refused ([`Reason::UnstableNormalization`]).
//!
//! # Backends
//!
//! * `nfc-tables` (default): NFC from compiled-in tables, Unicode
//!   [`UNICODE_VERSION`].
//! * `js` without `nfc-tables`, on wasm32: NFC from the JavaScript engine's
//!   `String.prototype.normalize`, so the module carries no normalization
//!   tables. [`enforce`] refuses with [`Refused::UnsupportedEngine`] when the
//!   engine predates [`NFC_BASELINE`]; check [`engine_supported`] before
//!   [`prepare`].
//!
//! Without a backend only [`refusal`] and the vocabulary are available.
//!
//! The crate is `no_std` and needs `alloc`.

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

mod error;
mod prepared;
mod property;
mod reason;
#[cfg(any(
    test,
    feature = "nfc-tables",
    all(feature = "js", target_arch = "wasm32")
))]
mod space;
#[allow(
    dead_code,
    reason = "NFC_BASELINE and ZS_NON_ASCII are read by tests, CANARY by the js backend"
)]
mod tables;

#[cfg(any(feature = "nfc-tables", all(feature = "js", target_arch = "wasm32")))]
mod nfc;
#[cfg(any(feature = "nfc-tables", all(feature = "js", target_arch = "wasm32")))]
mod profile;

pub use error::{Offense, Refused, UnsupportedEngine};
pub use prepared::Prepared;
pub use property::refusal;
pub use reason::Reason;

#[cfg(any(feature = "nfc-tables", all(feature = "js", target_arch = "wasm32")))]
pub use nfc::engine_supported;
#[cfg(any(feature = "nfc-tables", all(feature = "js", target_arch = "wasm32")))]
pub use profile::{enforce, prepare};

/// The Unicode version of the refusal table (and of the compiled-in NFC
/// tables, with `nfc-tables`). A different version is a minor release: it
/// changes what [`enforce`] refuses, never what [`prepare`] returns for an
/// accepted password.
pub const UNICODE_VERSION: &str = "17.0.0";

/// Code points newer than this Unicode version that NFC treats differently
/// on an engine that does not know them are refused, and a JavaScript
/// engine must know this version.
pub const NFC_BASELINE: &str = "12.0";

/// The generated table and the linked normalization tables are the same
/// Unicode version, or this does not compile.
#[cfg(feature = "nfc-tables")]
const _: () = {
    let linked = unicode_normalization::UNICODE_VERSION;
    let table = tables::UNICODE_VERSION;
    assert!(
        linked.0 == table.0 && linked.1 == table.1 && linked.2 == table.2,
        "unicode-normalization tables moved: regenerate src/tables.rs (tablegen/regen.sh)"
    );
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::{format, vec::Vec};

    #[test]
    fn version_constants_match_the_table() {
        let (a, b, c) = tables::UNICODE_VERSION;
        assert_eq!(UNICODE_VERSION, format!("{a}.{b}.{c}"));
        let (a, b) = tables::NFC_BASELINE;
        assert_eq!(NFC_BASELINE, format!("{a}.{b}"));
    }

    #[test]
    fn space_map_is_the_generated_zs_list() {
        let mapped: Vec<char> = (0x80..=0x10FFFF)
            .filter_map(char::from_u32)
            .filter(|&c| space::map_space(c) == ' ')
            .collect();
        assert_eq!(mapped, tables::ZS_NON_ASCII);
        assert_eq!(space::map_space(' '), ' ');
        assert_eq!(space::map_space('a'), 'a');
    }

    #[test]
    fn runs_are_well_formed() {
        assert_eq!(tables::RUNS[0] >> 5, 0);
        for w in tables::RUNS.windows(2) {
            assert!(w[0] >> 5 < w[1] >> 5, "starts ascend");
            assert_ne!(w[0] & 0x1F, w[1] & 0x1F, "adjacent runs differ");
        }
        for run in tables::RUNS {
            let code = (run & 0x1F) as u8;
            assert!(code == 0 || Reason::from_code(code).is_some());
        }
    }

    #[test]
    fn reason_codes_round_trip() {
        for (i, r) in Reason::ALL.iter().enumerate() {
            assert_eq!(*r as u8 as usize, i + 1);
            assert_eq!(Reason::from_code(*r as u8), Some(*r));
        }
        assert_eq!(Reason::from_code(0), None);
    }

    #[test]
    fn precedence_follows_rfc_8264_section_8() {
        let cases = [
            ('a', None),
            (' ', None),
            ('\u{00A0}', None), // Zs: FREE_PVAL
            ('\u{FB01}', None), // HasCompat: FREE_PVAL
            ('\u{2163}', None), // Nl: OtherLetterDigits
            ('\u{1F600}', None),
            ('\u{00DF}', None), // Exceptions PVALID
            ('\u{00B7}', None), // CONTEXTO, allowed here
            ('\u{0661}', None),
            ('\u{30FB}', None),
            ('\u{0640}', Some(Reason::Exception)),
            ('\u{303B}', Some(Reason::Exception)),
            ('\u{200C}', Some(Reason::JoinControl)),
            ('\u{200D}', Some(Reason::JoinControl)),
            ('\u{1100}', Some(Reason::OldHangulJamo)),
            ('\u{1160}', Some(Reason::OldHangulJamo)), // jamo before ignorable
            ('\u{200B}', Some(Reason::Ignorable)),
            ('\u{FE0F}', Some(Reason::Ignorable)),
            ('\u{E0041}', Some(Reason::Ignorable)),
            ('\u{FDD0}', Some(Reason::Ignorable)), // noncharacter, not unassigned
            ('\u{FFFF}', Some(Reason::Ignorable)),
            ('\t', Some(Reason::Control)),
            ('\u{7F}', Some(Reason::Control)),
            ('\u{0378}', Some(Reason::Unassigned)),
            ('\u{E0080}', Some(Reason::Unassigned)), // Default_Ignorable, but Unassigned comes first
            ('\u{2028}', Some(Reason::Disallowed)),
            ('\u{0600}', Some(Reason::Disallowed)),
            ('\u{E000}', Some(Reason::Disallowed)),
            ('\u{0EBA}', None), // the canary: Unicode 12.0, allowed
            ('\u{1ABF}', Some(Reason::UnstableNormalization)), // 13.0, Mn
        ];
        for (c, want) in cases {
            assert_eq!(refusal(c), want, "U+{:04X}", u32::from(c));
        }
    }

    #[cfg(feature = "nfc-tables")]
    #[test]
    fn canary_composes_with_compiled_tables() {
        assert!(engine_supported());
        let c = tables::CANARY;
        assert_eq!(
            prepare(&format!("a{c}\u{0301}")).as_str(),
            format!("\u{E1}{c}")
        );
    }

    #[cfg(feature = "nfc-tables")]
    #[test]
    fn prepared_debug_is_redacted() {
        let p = prepare("hunter2");
        assert_eq!(format!("{p:?}"), "Prepared(..)");
    }
}
