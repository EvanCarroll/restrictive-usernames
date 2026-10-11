//! The refusal table lookup.

use crate::reason::Reason;
use crate::tables::RUNS;

/// Why `c` is refused in a prepared password, or `None` if it is allowed.
///
/// This is the class [`crate::enforce`] checks, for Unicode
/// [`crate::UNICODE_VERSION`]: RFC 8264 FreeformClass with this crate's
/// deviations (see the crate documentation).
pub fn refusal(c: char) -> Option<Reason> {
    let cp = u32::from(c);
    // RUNS[0] starts at U+0000, so the run containing cp is at i - 1 >= 0.
    let i = RUNS.partition_point(|&run| run >> 5 <= cp);
    Reason::from_code((RUNS[i - 1] & 0x1F) as u8)
}
