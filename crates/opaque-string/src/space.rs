//! The RFC 8265 §4.2.2 additional mapping rule.

/// Maps a non-ASCII space (General Category Zs) to U+0020.
///
/// Written out, not generated, and frozen: `prepare` must give the same
/// bytes for an accepted password forever, so a space added in a future
/// Unicode version is refused by `enforce` instead of mapped. A test holds
/// this list to the generated one for the current version.
pub(crate) const fn map_space(c: char) -> char {
    match c {
        '\u{00A0}'
        | '\u{1680}'
        | '\u{2000}'..='\u{200A}'
        | '\u{202F}'
        | '\u{205F}'
        | '\u{3000}' => ' ',
        _ => c,
    }
}
