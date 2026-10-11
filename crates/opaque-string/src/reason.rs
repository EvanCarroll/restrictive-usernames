//! Why a code point is refused.

use core::fmt;

/// The rule that refuses a code point in [`crate::enforce`].
///
/// The discriminants are the codes in the generated table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum Reason {
    /// Not assigned in Unicode [`crate::UNICODE_VERSION`] (RFC 8264 §9.10).
    Unassigned = 1,
    /// General Category Cc (RFC 8264 §9.12).
    Control = 2,
    /// Default_Ignorable_Code_Point or a noncharacter (RFC 8264 §9.13):
    /// invisible characters, variation selectors, tags.
    Ignorable = 3,
    /// Conjoining Hangul jamo (RFC 8264 §9.9). A modern syllable typed as
    /// jamo is accepted, because NFC composes it first.
    OldHangulJamo = 4,
    /// ZWJ or ZWNJ. The RFC allows them in context (CONTEXTJ); this crate
    /// refuses them everywhere.
    JoinControl = 5,
    /// DISALLOWED by the RFC 5892 §2.6 Exceptions (e.g. U+0640 ARABIC
    /// TATWEEL).
    Exception = 6,
    /// In no FreeformClass category: General Category Cf, Co, Zl, Zp
    /// (RFC 8264 §8, the final "Else").
    Disallowed = 7,
    /// Newer than [`crate::NFC_BASELINE`] and treated differently by NFC on
    /// an older engine (a combining mark, a decomposable character, or part
    /// of one). Refused so that every engine produces the same bytes.
    UnstableNormalization = 8,
}

impl Reason {
    /// Every reason, in code order.
    pub const ALL: &'static [Reason] = &[
        Reason::Unassigned,
        Reason::Control,
        Reason::Ignorable,
        Reason::OldHangulJamo,
        Reason::JoinControl,
        Reason::Exception,
        Reason::Disallowed,
        Reason::UnstableNormalization,
    ];

    /// The snake_case name, e.g. `"join_control"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Reason::Unassigned => "unassigned",
            Reason::Control => "control",
            Reason::Ignorable => "ignorable",
            Reason::OldHangulJamo => "old_hangul_jamo",
            Reason::JoinControl => "join_control",
            Reason::Exception => "exception",
            Reason::Disallowed => "disallowed",
            Reason::UnstableNormalization => "unstable_normalization",
        }
    }

    /// The reason a table code stands for; `None` for 0 (allowed).
    pub(crate) const fn from_code(code: u8) -> Option<Reason> {
        Some(match code {
            1 => Reason::Unassigned,
            2 => Reason::Control,
            3 => Reason::Ignorable,
            4 => Reason::OldHangulJamo,
            5 => Reason::JoinControl,
            6 => Reason::Exception,
            7 => Reason::Disallowed,
            8 => Reason::UnstableNormalization,
            _ => return None,
        })
    }
}

/// Short English, for consumers without localization.
impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Reason::Unassigned => "unassigned",
            Reason::Control => "control character",
            Reason::Ignorable => "invisible or noncharacter",
            Reason::OldHangulJamo => "conjoining Hangul jamo",
            Reason::JoinControl => "zero-width joiner or non-joiner",
            Reason::Exception => "disallowed by RFC 5892",
            Reason::Disallowed => "disallowed",
            Reason::UnstableNormalization => "too new to normalize the same everywhere",
        })
    }
}
