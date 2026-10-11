//! Confusable-skeleton keys for uniqueness.

use std::borrow::Cow;

use unicode_bidi::{BidiClass, BidiInfo, bidi_class};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::structural::prep;

fn nfkc(s: &str) -> String {
    s.nfkc().collect()
}

/// Full Unicode case folding, CaseFolding.txt statuses C + F (`ß` → `ss`).
fn fold(s: &str) -> String {
    caseless::default_case_fold_str(s)
}

/// UTS #39 §4 skeleton: NFD, map each code point through confusables.txt,
/// NFD again.
fn skeleton(s: &str) -> String {
    unicode_security::skeleton(s).collect()
}

/// The ingress the keys share: `nfkc(prep(x))`.
fn prepared(input: &str) -> String {
    nfkc(&prep(input))
}

/// The case-insensitive half: fold, skeleton, fold.
fn ci(s: &str) -> String {
    fold(&skeleton(&fold(s)))
}

/// The as-typed half: skeleton, then fold.
fn cs(s: &str) -> String {
    fold(&skeleton(s))
}

/// Whether the UBA can move anything in a string holding `c`: a
/// right-to-left letter (R, AL), an Arabic number (AN), or an explicit
/// right-to-left or first-strong embedding, override or isolate. Without
/// one, every code point resolves to an even level and display order is
/// logical order (`fast_path_is_exact` checks every code point).
fn may_reorder(c: char) -> bool {
    matches!(
        bidi_class(c),
        BidiClass::R
            | BidiClass::AL
            | BidiClass::AN
            | BidiClass::RLE
            | BidiClass::RLO
            | BidiClass::RLI
            | BidiClass::FSI
    )
}

/// UAX #9 visual order, as `<bdi>`, `dir="auto"` and U+2068 FSI .. U+2069
/// PDI display the string: each paragraph's direction from its first strong
/// code point (rules P2, P3), one line per paragraph, combining marks kept
/// after their base in right-to-left runs (L3). Bidi_Mirrored glyph
/// substitution (L4) is not applied; the Nickname profile has no mirrored
/// code points.
fn visual(s: &str) -> Cow<'_, str> {
    if !s.chars().any(may_reorder) {
        return Cow::Borrowed(s);
    }
    reorder(s)
}

/// Each paragraph reordered as one line, its separator (Bidi_Class B) kept
/// after it rather than reordered into it.
fn reorder(s: &str) -> Cow<'_, str> {
    let info = BidiInfo::new(s, None);
    let mut out = String::with_capacity(s.len());
    for para in &info.paragraphs {
        let mut line = para.range.clone();
        let sep = s[line.clone()]
            .chars()
            .next_back()
            .filter(|&c| bidi_class(c) == BidiClass::B);
        if let Some(c) = sep {
            line.end -= c.len_utf8();
        }
        if !line.is_empty() {
            let (levels, runs) = info.visual_runs(para, line);
            for run in runs {
                if levels[run.start].is_rtl() {
                    push_reversed(&mut out, &s[run]);
                } else {
                    out.push_str(&s[run]);
                }
            }
        }
        out.extend(sep);
    }
    Cow::Owned(out)
}

/// A right-to-left run in display order with UAX #9 rule L3 applied:
/// combining sequences (a base and the General Category M* code points after
/// it) are reversed, the code points inside each are not, so every mark
/// still follows its base, as UTS #39 bidiSkeleton requires. Reversing code
/// points instead would put `بىٔب` (U+0654 after U+0649) in an order its
/// look-alike `بئب` never reaches after the skeleton mapping.
fn push_reversed(out: &mut String, run: &str) {
    let mut end = run.len();
    let mut start = end;
    for (i, c) in run.char_indices().rev() {
        start = i;
        if !is_combining_mark(c) {
            out.push_str(&run[start..end]);
            end = start;
        }
    }
    // Marks with no base before them in the run stay as one sequence.
    out.push_str(&run[start..end]);
}

/// The case-insensitive confusable key: `fold(skeleton(fold(nfkc(prep(x)))))`.
///
/// `prep` is the RFC 8266 §2.2 space mapping, collapse and trim, so raw
/// lookup text and storage forms key alike. NFKC (UAX #15) folds
/// compatibility variants (`ﬁ`, fullwidth), case folding is full (C+F,
/// Unicode §3.13), and the UTS #39 §4 skeleton maps confusables to one
/// prototype. Folding again after the skeleton makes the key case-stable.
/// Two names collide when their keys are equal: `ADMIN` / `admin`,
/// `pаypal` (Cyrillic а) / `paypal`, `Straße` / `Strasse`.
///
/// Pure function of the input and the compiled-in tables, hence `IMMUTABLE`
/// in SQL; a table change is a [`crate::POLICY_VERSION`] bump and a REINDEX.
pub fn key_ci(input: &str) -> String {
    ci(&prepared(input))
}

/// The as-typed confusable key: `fold(skeleton(nfkc(prep(x))))`.
///
/// Skeleton before folding: catches the pairs whose confusability depends on
/// case, which [`key_ci`] folds away first: `Iris` / `lris` (capital I vs
/// small L), `Bob` / `B0b` (zero vs capital O). Use both keys; a name is
/// available when neither collides.
pub fn key_cs(input: &str) -> String {
    cs(&prepared(input))
}

/// [`key_ci`] of the displayed form:
/// `fold(skeleton(fold(visual(nfkc(prep(x))))))`.
///
/// `visual` is UAX #9 display order with the paragraph direction taken from
/// the first strong code point, which is how `<bdi>`, `dir="auto"` and
/// U+2068 FSI .. U+2069 PDI render a name. Two names that look alike under
/// that rendering collide even when their stored order differs:
/// `Dan אבי` / `אבי Dan` both display as `Dan יבא`. Names that display
/// differently do not, even when their stored-order keys would:
/// `ADMIN ٢ וו` (Arabic-Indic digit, displayed `ADMIN וו ٢`) / `ADMIN ۲ וו`
/// (Extended Arabic-Indic digit, displayed in place).
///
/// For a name with no right-to-left code point the display order is the
/// stored order, so this equals [`key_ci`] and costs the same.
///
/// The key holds only where every surface renders names isolated with
/// first-strong direction (README, "Rendering contract"). Pair it with
/// [`key_visual_cs`]: a name is available when neither collides.
pub fn key_visual_ci(input: &str) -> String {
    ci(&visual(&prepared(input)))
}

/// [`key_cs`] of the displayed form: `fold(skeleton(visual(nfkc(prep(x)))))`.
///
/// Catches case-dependent look-alikes in display order (`Iris אבי` /
/// `אבי lris`), which [`key_visual_ci`] folds away first, as [`key_cs`] does
/// for [`key_ci`].
pub fn key_visual_cs(input: &str) -> String {
    cs(&visual(&prepared(input)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shortcut in [`visual`] never changes the result: for every code
    /// point the shortcut admits, alone and between letters, digits and
    /// spaces, the full UBA leaves the string as it is.
    #[test]
    fn fast_path_is_exact() {
        let mut failures = Vec::new();
        for c in (0..=0x10FFFF).filter_map(char::from_u32) {
            if may_reorder(c) {
                continue;
            }
            for s in [format!("{c}"), format!("a{c} 1{c}b"), format!("{c}2 {c}")] {
                if reorder(&s) != s {
                    failures.push(format!("U+{:04X}", c as u32));
                    break;
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join(" "));
    }

    #[test]
    fn displayed_order() {
        assert_eq!(
            visual("Dan \u{5D0}\u{5D1}\u{5D9}"),
            "Dan \u{5D9}\u{5D1}\u{5D0}"
        );
        assert_eq!(
            visual("\u{5D0}\u{5D1}\u{5D9} Dan"),
            "Dan \u{5D9}\u{5D1}\u{5D0}"
        );
        // First strong code point is Hebrew: an RTL paragraph.
        assert_eq!(visual("1 \u{5D0}"), "\u{5D0} 1");
        assert!(matches!(visual("Jean-Luc 2"), Cow::Borrowed(_)));
        // One line per paragraph.
        assert_eq!(
            visual("\u{5D0}\u{5D1}\n\u{5D2}\u{5D3}"),
            "\u{5D1}\u{5D0}\n\u{5D3}\u{5D2}"
        );
    }

    #[test]
    fn marks_follow_their_base_in_rtl_runs() {
        // U+0628 U+0649 U+0654 U+0628: the hamza stays after the yeh.
        assert_eq!(
            visual("\u{628}\u{649}\u{654}\u{628}"),
            "\u{628}\u{649}\u{654}\u{628}"
        );
        // Hebrew: alef with qamats, bet; reversed by sequence.
        assert_eq!(visual("\u{5D0}\u{5B8}\u{5D1}"), "\u{5D1}\u{5D0}\u{5B8}");
        // A leading mark with no base in the run is kept as is.
        assert_eq!(visual("\u{5B8}\u{5D0}\u{5D1}"), "\u{5D1}\u{5D0}\u{5B8}");
    }

    /// Every nonspacing mark (Bidi_Class NSM takes the level of its base,
    /// UAX #9 W1) after every right-to-left base stays after that base.
    #[test]
    fn rtl_marks_never_precede_their_base() {
        let bases = [
            '\u{5D0}', '\u{628}', '\u{649}', '\u{62D}', '\u{710}', '\u{7CA}',
        ];
        let mut failures = Vec::new();
        for m in (0..=0x10FFFF)
            .filter_map(char::from_u32)
            .filter(|&c| is_combining_mark(c) && bidi_class(c) == BidiClass::NSM)
        {
            for b in bases {
                let s = format!("{b}{m}{b}");
                if visual(&s) != format!("{b}{b}{m}") {
                    failures.push(format!("U+{:04X} U+{:04X}", b as u32, m as u32));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join(", "));
    }

    #[test]
    fn visual_keys_equal_stored_keys_without_rtl() {
        for s in [
            "Bob",
            "ADMIN",
            "Iris",
            "St. John",
            "p\u{430}ypal",
            "日本語",
            "Straße",
        ] {
            assert_eq!(key_visual_ci(s), key_ci(s), "{s}");
            assert_eq!(key_visual_cs(s), key_cs(s), "{s}");
        }
    }
}
