//! Error types.
//!
//! Nothing here holds or prints the password. A refusal names the offending
//! code point as `U+XXXX`: a code point that is refused cannot occur in any
//! accepted password, so naming it gives nothing away.

use core::fmt;
use core::num::NonZeroUsize;

use crate::reason::Reason;

/// [`crate::enforce`] refused the password.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Refused {
    /// Nothing is left after preparation (RFC 8265 §4.2.2).
    #[error("password is empty")]
    Empty,
    /// The prepared password holds code points the profile refuses.
    #[error("password contains {count} disallowed code point(s); the first is {first}")]
    Disallowed {
        /// The first refused code point.
        first: Offense,
        /// How many code points are refused.
        count: NonZeroUsize,
    },
    /// The JavaScript engine's normalization is older than the baseline.
    #[error(transparent)]
    UnsupportedEngine(#[from] UnsupportedEngine),
}

/// One refused code point of a prepared password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Offense {
    /// Position in the prepared password, in code points.
    pub index: usize,
    /// The code point.
    pub code_point: char,
    /// Why it is refused.
    pub reason: Reason,
}

/// `U+XXXX (reason)`: never the character itself, which may be a control
/// or a bidirectional override, and never its position.
impl fmt::Display for Offense {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "U+{:04X} ({})", u32::from(self.code_point), self.reason)
    }
}

/// The JavaScript engine's `String.prototype.normalize` does not know
/// Unicode [`crate::NFC_BASELINE`], so it could produce different bytes than
/// other devices. Only the `js` backend reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "the JavaScript engine's Unicode normalization predates Unicode {}",
    crate::NFC_BASELINE
)]
pub struct UnsupportedEngine;
