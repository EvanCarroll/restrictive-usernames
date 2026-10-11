//! The three kinds of name.

use serde::{Deserialize, Serialize};

/// Which profile a name is checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// An account name. Shape of the PRECIS IdentifierClass (RFC 8264 §4.2,
    /// RFC 8265 §3): UTS #39 Identifier_Type Recommended, Limited_Use (and
    /// `_`), restricted to letters, digits and marks (XID_Continue, UAX #31),
    /// first code point a letter, no whitespace, one word, UTS #39 §5.2
    /// Highly Restrictive.
    Username,
    /// A display name. Shape of the PRECIS Nickname profile (RFC 8266 §2):
    /// the identifier profile including its Inclusion punctuation
    /// (`' - . :`, U+00B7, U+05F3, U+30FB, ...), words separated by single
    /// U+0020, Highly Restrictive per word.
    Nickname,
    /// A room, scene or avatar name. Refuses UTS #39 Identifier_Type
    /// Not_Character, Default_Ignorable and Deprecated, and controls;
    /// symbols and emoji are allowed. No script or confusable rule.
    Label,
}

impl Kind {
    /// The serde name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Username => "username",
            Kind::Nickname => "nickname",
            Kind::Label => "label",
        }
    }
}
