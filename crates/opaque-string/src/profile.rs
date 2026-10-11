//! The two entry points.

use core::num::NonZeroUsize;

use crate::error::{Offense, Refused, UnsupportedEngine};
use crate::nfc::{engine_supported, mapped_nfc};
use crate::prepared::Prepared;
use crate::property::refusal;

/// Login: the bytes a password stands for. Maps non-ASCII spaces to U+0020,
/// then applies NFC (RFC 8265 §4.2.2 rules 2 and 4). Refuses nothing.
///
/// For any password [`enforce`] accepted, the output is the same on every
/// platform and every later release, so logging in never depends on which
/// device or version registered the password. Refusing nothing at login
/// keeps it that way when a later release refuses more at registration.
///
/// On the `js` backend, check [`crate::engine_supported`] first: an engine
/// older than [`crate::NFC_BASELINE`] can produce different bytes.
pub fn prepare(input: &str) -> Prepared {
    Prepared(mapped_nfc(input))
}

/// Registration and password change: [`prepare`], then refuse code points
/// outside the profile and an empty result (RFC 8265 §4.2.2).
///
/// The class is checked on the prepared password, so
/// `enforce(prepare(x).as_str())` and `enforce(x)` agree, and the accepted
/// value is exactly what [`prepare`] gives at login.
pub fn enforce(input: &str) -> Result<Prepared, Refused> {
    if !engine_supported() {
        return Err(UnsupportedEngine.into());
    }
    let prepared = prepare(input);
    let mut first = None;
    let mut count = 0usize;
    for (index, code_point) in prepared.as_str().chars().enumerate() {
        if let Some(reason) = refusal(code_point) {
            count += 1;
            first.get_or_insert(Offense {
                index,
                code_point,
                reason,
            });
        }
    }
    if let (Some(first), Some(count)) = (first, NonZeroUsize::new(count)) {
        return Err(Refused::Disallowed { first, count });
    }
    if prepared.is_empty() {
        return Err(Refused::Empty);
    }
    Ok(prepared)
}
