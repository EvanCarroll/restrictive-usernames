//! The prepared password.

use alloc::string::String;
use core::fmt;

use zeroize::Zeroize;

/// A prepared password: the bytes to hand to OPAQUE or a password hash.
///
/// The buffer is zeroed when it is dropped. It has no `Display`, and its
/// `Debug` prints `Prepared(..)`, so it cannot end up in a log by accident.
/// Copies made elsewhere (the caller's input, a JavaScript string, internal
/// buffers of the normalizer) are not covered.
pub struct Prepared(pub(crate) String);

impl Prepared {
    /// The prepared password as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The prepared password as UTF-8 bytes: the input to OPAQUE or a
    /// password hash.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Whether the prepared password is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl AsRef<[u8]> for Prepared {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<str> for Prepared {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for Prepared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Prepared(..)")
    }
}

impl Drop for Prepared {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
