//! The acceptance corpus, as data.
//!
//! The crate's unit tests and the extension's `#[pg_test]`s iterate these
//! same tables, so the Rust and SQL surfaces are held to one expectation.
//! Expected rule lists are exact (every failed rule, in [`Rule::ALL`] order).

use crate::kind::Kind;
use crate::rule::Rule;

/// A storage-form input and the exact result of `check`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Case {
    /// The name, as stored.
    pub input: &'static str,
    /// The profile it is checked against, under [`crate::Policy::for_kind`].
    pub kind: Kind,
    /// The exact `check` result; empty means accepted.
    pub rules: &'static [Rule],
}

const fn case(input: &'static str, kind: Kind, rules: &'static [Rule]) -> Case {
    Case { input, kind, rules }
}

use Kind::{Label as L, Nickname as N, Username as U};

const MIXED: &[Rule] = &[Rule::MixedScript];
const DISALLOWED: &[Rule] = &[Rule::DisallowedChar];
const FIRST: &[Rule] = &[Rule::FirstNotLetter];
const OK: &[Rule] = &[];

/// क followed by 25 × U+0941 DEVANAGARI VOWEL SIGN U (Mn, ccc 0).
pub const DEVANAGARI_25_U: &str = "\u{915}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}\u{941}";

/// `check` cases.
pub const CHECK: &[Case] = &[
    // Accepted nicknames.
    case("Jörg", N, OK),
    case("Ωμέγα", N, OK),
    case("नमस्ते", N, OK),
    case("한글", N, OK),
    case("日本語", N, OK),
    case("العربية", N, OK),
    case("עברית", N, OK),
    case("ローマ字Latin", N, OK),
    case("한국어Latin", N, OK),
    case("中文Latin", N, OK),
    case("Иван Smith", N, OK),
    case("Mountain Host", N, OK),
    case("Bob 2", N, OK),
    case("O'Brien", N, OK),
    case("Jean-Luc", N, OK),
    case("ᏣᎳᎩ", N, OK),     // Cherokee, Limited_Use
    case("ᓀᐦᐃᔭᐍᐏᐣ", N, OK), // Canadian Aboriginal Syllabics, Limited_Use
    case("\u{E1}", N, OK),  // á, precomposed
    // `.` is UTS #39 Inclusion, so it is in the Nickname profile.
    case("Dr. Who", N, OK),
    case("idris", N, OK),
    case("ident:x", N, OK),
    // Accepted usernames.
    case("xX_Dark_Xx", U, OK),
    case("Jörg", U, OK),
    case("ᏣᎳᎩ", U, OK),
    case("नमस्ते", U, OK),
    // Thai with U+0E4C THANTHAKHAT: a mark with ccc 0 that is not Alphabetic.
    case("ศาสตร์", U, OK),
    case("日本語", U, OK),
    // Mixed script (UTS #39 §5.2).
    case("\u{410}dmin", N, MIXED),               // Cyrillic А
    case("\u{3A9}mega", N, MIXED),               // Greek Ω
    case("\u{410}dmin Smith", N, MIXED),         // checked per word
    case("Bob\u{662}", N, MIXED),                // Arabic-Indic digit in a Latin word
    case("\u{628}\u{648}\u{628}\u{662}", N, OK), // the same digit in an Arabic word
    // Outside the identifier profile.
    case("🇺🇸", N, DISALLOWED),
    case("👋🏽", N, DISALLOWED),
    case("★★★", N, DISALLOWED),
    case("🤖 Helper", N, DISALLOWED),
    case("Bob!", N, DISALLOWED),
    case("ǅemal", N, DISALLOWED), // Not_NFKC titlecase digraph
    case("ﬁsh", N, DISALLOWED),   // Not_NFKC ligature
    case("Ｆｕｌｌｗｉｄｔｈ", N, DISALLOWED),
    case("Ⅷ", N, DISALLOWED),
    case("𝓑𝓸𝓫", N, DISALLOWED),
    case("❤\u{FE0F}", N, DISALLOWED), // VS16 is Default_Ignorable
    case("👨\u{200D}👩\u{200D}👧", N, DISALLOWED), // ZWJ
    case(
        "🏴\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}",
        N,
        DISALLOWED,
    ), // tags
    // Javanese pada: Limited_Use script, but Not_XID punctuation.
    // Also Javanese + Latin in one word.
    case("꧁Bob꧂", N, &[Rule::DisallowedChar, Rule::MixedScript]),
    case("abc\u{202E}def", N, DISALLOWED),
    case("\u{2066}abc", N, DISALLOWED),
    // ZWNJ is Default_Ignorable; RFC 5892 CONTEXTJ is not implemented.
    case("علی\u{200C}رضا", N, DISALLOWED),
    // Marks (General Category M*, NFD form).
    case("\u{301}", N, &[Rule::DisallowedChar, Rule::NoBase]),
    case(
        "a\u{301}\u{301}\u{301}\u{301}\u{301}",
        N,
        &[Rule::NotNfc, Rule::MarkRun { max: 4 }],
    ),
    // á + 3 distinct marks: NFD a + 4 distinct marks, exactly max_mark_run.
    case("\u{E1}\u{302}\u{303}\u{304}", N, OK),
    // á + 4 distinct marks: 5 in NFD.
    case(
        "\u{E1}\u{302}\u{303}\u{304}\u{306}",
        N,
        &[Rule::MarkRun { max: 4 }],
    ),
    // á + U+0301: the same mark twice in a row in NFD.
    case("\u{E1}\u{301}", N, &[Rule::MarkRun { max: 4 }]),
    case(
        "\u{E1}\u{301}\u{301}\u{301}",
        N,
        &[Rule::MarkRun { max: 4 }],
    ),
    // Marks with canonical combining class 0 count (Devanagari, Thai, Khmer).
    case(DEVANAGARI_25_U, U, &[Rule::MarkRun { max: 4 }]),
    case(DEVANAGARI_25_U, N, &[Rule::MarkRun { max: 4 }]),
    case(
        "\u{E01}\u{E34}\u{E34}\u{E34}\u{E34}\u{E34}\u{E34}\u{E34}\u{E34}\u{E34}\u{E34}",
        N,
        &[Rule::MarkRun { max: 4 }],
    ),
    case(
        "\u{E34}\u{E34}\u{E34}",
        N,
        &[Rule::DisallowedChar, Rule::MarkRun { max: 4 }, Rule::NoBase],
    ),
    case(
        "\u{1780}\u{17B7}\u{17B7}\u{17B7}",
        N,
        &[Rule::MarkRun { max: 4 }],
    ),
    case("\u{17B7}", N, &[Rule::DisallowedChar, Rule::NoBase]),
    // A mark must follow its base.
    case("\u{301}abc", N, &[Rule::NoBase]),
    case("a \u{301}bc", N, &[Rule::NoBase]),
    // Ordinary mark sequences stay legal.
    case("\u{939}\u{948}\u{902}", N, OK), // हैं
    case("\u{E01}\u{E35}\u{E48}", N, OK), // กี่
    // A nickname needs a letter or digit.
    case(".", N, DISALLOWED),
    case("...", N, DISALLOWED),
    case("'", N, DISALLOWED),
    case(":", N, DISALLOWED),
    case("-", N, DISALLOWED),
    case("\u{B7}", N, DISALLOWED),
    case("_", N, DISALLOWED),
    case(". .", N, DISALLOWED),
    case("2", N, OK),
    // Structural.
    case("   ", N, &[Rule::Empty]),
    case("\u{3000}", N, &[Rule::Empty]),
    // `:` is UTS #39 Inclusion: `id:x` is an ordinary nickname, `#` is not.
    case("id:x", N, OK),
    case("ID:x", N, OK),
    case(" id:x", N, &[Rule::EdgeWhitespace]),
    case("#x", N, DISALLOWED),
    case("Bob  2", N, &[Rule::Whitespace]),
    case("Bob\u{A0}2", N, &[Rule::Whitespace]),
    case("Bob\t2", N, &[Rule::Whitespace, Rule::ControlChar]),
    case(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        N,
        &[Rule::Length { min: 1, max: 32 }],
    ),
    case("_user", U, FIRST),
    case("9lives", U, FIRST),
    case("user name", U, &[Rule::Whitespace]),
    case("Jean-Luc", U, DISALLOWED),
    case("ab", U, &[Rule::Length { min: 3, max: 32 }]),
    case("id:x", U, DISALLOWED),
    case("user-name", U, DISALLOWED),
    // Labels.
    case("🎉 Party", L, OK),
    case("Room ★ 2", L, OK),
    case("Ｆｕｌｌｗｉｄｔｈ", L, OK),
    case("\u{410}dmin", L, OK),
    case("id:x", L, OK),
    case("❤\u{FE0F} Room", L, DISALLOWED),
    case("Room\u{200B}", L, DISALLOWED),
    case("\u{E000}", L, DISALLOWED), // private use: no Identifier_Type
    case("e\u{301}", L, &[Rule::NotNfc]),
    case(" Lobby", L, &[Rule::EdgeWhitespace]),
    case("a\u{A0}b", L, &[Rule::Whitespace]),
    case("a\u{0}b", L, &[Rule::ControlChar]),
    case("\u{301}", L, &[Rule::NoBase]),
    case(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        L,
        &[Rule::Length { min: 1, max: 50 }],
    ),
];

/// A raw (typed) input and the exact result of `normalize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizeCase {
    /// The name, as typed.
    pub input: &'static str,
    /// The profile, under [`crate::Policy::for_kind`].
    pub kind: Kind,
    /// `Ok(storage form)` or `Err(exact rules)`.
    pub expected: Result<&'static str, &'static [Rule]>,
}

const fn norm(
    input: &'static str,
    kind: Kind,
    expected: Result<&'static str, &'static [Rule]>,
) -> NormalizeCase {
    NormalizeCase {
        input,
        kind,
        expected,
    }
}

/// The 19-code-point Zalgo string (Z, seventeen combining marks, a).
pub const ZALGO: &str = "Z\u{334}\u{322}\u{31B}\u{316}\u{32D}\u{319}\u{339}\u{32E}\u{313}\u{313}\u{30D}\u{34B}\u{309}\u{306}\u{303}\u{315}\u{35D}a";

/// `normalize` cases.
pub const NORMALIZE: &[NormalizeCase] = &[
    norm("  Mountain\u{3000}\u{3000}Host ", N, Ok("Mountain Host")),
    norm("Bob\u{A0}\u{2009}2", N, Ok("Bob 2")),
    norm("e\u{301}", N, Ok("\u{E9}")),
    norm("Jo\u{308}rg", U, Ok("J\u{F6}rg")),
    norm(" Lobby  ", L, Ok("Lobby")),
    // NFC composes one mark into á; NFD still counts five.
    norm(
        "a\u{301}\u{301}\u{301}\u{301}\u{301}",
        N,
        Err(&[Rule::MarkRun { max: 4 }]),
    ),
    norm(
        ZALGO,
        N,
        Err(&[Rule::DisallowedChar, Rule::MarkRun { max: 4 }]),
    ),
    norm(" id:x", N, Ok("id:x")),
    norm(" #x", N, Err(DISALLOWED)),
    norm("\u{A0}\u{A0}", N, Err(&[Rule::Empty])),
    norm(" bob", U, Err(&[Rule::FirstNotLetter, Rule::Whitespace])),
];

/// A pair of names and whether each key of a pair of keys makes them
/// collide: [`KEY_PAIRS`] holds `key_ci` / `key_cs`, [`VISUAL_KEY_PAIRS`]
/// holds `key_visual_ci` / `key_visual_cs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyPair {
    /// First name.
    pub a: &'static str,
    /// Second name.
    pub b: &'static str,
    /// The case-insensitive keys of `a` and `b` are equal.
    pub equal_ci: bool,
    /// The as-typed keys of `a` and `b` are equal.
    pub equal_cs: bool,
}

const fn pair(a: &'static str, b: &'static str, equal_ci: bool, equal_cs: bool) -> KeyPair {
    KeyPair {
        a,
        b,
        equal_ci,
        equal_cs,
    }
}

/// `key_ci` / `key_cs` pairs: equal on ci only, equal on cs only, equal on
/// both, differ on both. None has a right-to-left code point, so the
/// display-order keys agree with these.
pub const KEY_PAIRS: &[KeyPair] = &[
    pair("ADMIN", "admin", true, false),
    // The outer fold of key_ci maps the skeleton's `O` (from `0`) back to
    // `o`, so this pair collides on both keys. (`skeleton(fold(..))`
    // without the outer fold keeps the `O`, and the ci keys differ.)
    pair("Bob", "B0b", true, true),
    pair("Iris", "lris", false, true),
    pair("p\u{430}ypal", "paypal", true, true),
    pair("ﬁsh", "fish", true, true),
    pair("Ｆｕｌｌ", "Full", true, true),
    pair("Straße", "Strasse", true, true),
    pair("Mountain\u{3000}Host", "Mountain Host", true, true),
    pair("Jörg", "Jorg", false, false),
    pair("İstanbul", "istanbul", false, false),
    pair("vv", "w", false, false),
    pair("cl", "d", false, false),
];

/// `key_visual_ci` / `key_visual_cs` pairs: names stored in different orders
/// that display alike under first-strong isolation collide; names that
/// display differently do not.
pub const VISUAL_KEY_PAIRS: &[KeyPair] = &[
    // Words of opposite direction, swapped: both display `Dan יבא`.
    pair(
        "Dan \u{5D0}\u{5D1}\u{5D9}",
        "\u{5D0}\u{5D1}\u{5D9} Dan",
        true,
        true,
    ),
    pair(
        "Bob \u{5E9}\u{5DC}\u{5D5}\u{5DD}",
        "\u{5E9}\u{5DC}\u{5D5}\u{5DD} Bob",
        true,
        true,
    ),
    pair(
        "Ali \u{645}\u{62D}\u{645}\u{62F}",
        "\u{645}\u{62D}\u{645}\u{62F} Ali",
        true,
        true,
    ),
    pair(
        "Bob 2 \u{5E9}\u{5DC}\u{5D5}\u{5DD}",
        "\u{5E9}\u{5DC}\u{5D5}\u{5DD} Bob 2",
        true,
        true,
    ),
    // Reordered inside a word of an LTR paragraph: `2 Dan 1-א`.
    pair("2 Dan \u{5D0}-1", "2 Dan 1-\u{5D0}", true, true),
    pair(
        "2 Bob \u{639}\u{644}\u{6CC}-2",
        "2 Bob 2-\u{639}\u{644}\u{6CC}",
        true,
        true,
    ),
    // Swapped and case-varied: ci only.
    pair(
        "ADMIN \u{5D0}\u{5D1}\u{5D9}",
        "\u{5D0}\u{5D1}\u{5D9} admin",
        true,
        false,
    ),
    // Swapped and case-dependent look-alike (capital I, small l): cs only.
    pair(
        "Iris \u{5D0}\u{5D1}\u{5D9}",
        "\u{5D0}\u{5D1}\u{5D9} lris",
        false,
        true,
    ),
    // Spacing is mapped before reordering (raw text: `normalize` maps it).
    pair(
        "Dan\u{3000}\u{5D0}\u{5D1}\u{5D9}",
        "\u{5D0}\u{5D1}\u{5D9} Dan",
        true,
        true,
    ),
    // The period moves: `Dan Jr. יבא` vs `.Dan Jr יבא`.
    pair(
        "Dan Jr. \u{5D0}\u{5D1}\u{5D9}",
        "\u{5D0}\u{5D1}\u{5D9} Dan Jr.",
        false,
        false,
    ),
    // Different paragraph directions: `א 1` vs `1 א`.
    pair("1 \u{5D0}", "\u{5D0} 1", false, false),
    // Confusable digits of different Bidi_Class display on opposite sides
    // of the Hebrew word (`ADMIN וו ٢` vs `ADMIN ۲ וו`): the stored-order
    // keys collide, the display-order keys do not.
    pair(
        "ADMIN \u{662} \u{5D5}\u{5D5}",
        "ADMIN \u{6F2} \u{5D5}\u{5D5}",
        false,
        false,
    ),
    // UAX #9 L3: in a right-to-left run a mark stays after its base, so
    // the skeleton's `ئ` -> `ىٔ` and the typed `ىٔ` agree in display order
    // as they do in stored order.
    pair(
        "\u{628}\u{626}\u{628}",
        "\u{628}\u{649}\u{654}\u{628}",
        true,
        true,
    ),
    pair(
        "\u{628}\u{681}\u{628}",
        "\u{628}\u{62D}\u{654}\u{628}",
        true,
        true,
    ),
    // Word order is not identity in one direction.
    pair("John Smith", "Smith John", false, false),
    // A reversed RTL spelling is another name.
    pair(
        "\u{5D0}\u{5D1}\u{5D9}",
        "\u{5D9}\u{5D1}\u{5D0}",
        false,
        false,
    ),
    // Whole-word confusables still collide next to an RTL word: Cyrillic
    // `Сосо` and Latin `Coco`.
    pair(
        "\u{421}\u{43E}\u{441}\u{43E} \u{5D0}\u{5D1}\u{5D9}",
        "\u{5D0}\u{5D1}\u{5D9} Coco",
        true,
        true,
    ),
];
