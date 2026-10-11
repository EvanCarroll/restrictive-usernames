//! Generates `crates/opaque-string/src/tables.rs`: the RFC 8265 §4.2
//! OpaqueString refusal table (the RFC 8264 FreeformClass with this crate's
//! deviations) for one pinned Unicode version. Run through `regen.sh`, which
//! fetches and checksums the UCD files first.

mod error;
mod ucd;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use clap::Parser;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::{canonical_combining_class, compose, decompose_canonical};

use crate::error::Error;
use crate::ucd::{CODE_SPACE, Ucd};

/// Generate the OpaqueString refusal table from the UCD.
#[derive(Parser)]
struct Args {
    /// Directory holding UnicodeData.txt, PropList.txt,
    /// DerivedCoreProperties.txt, HangulSyllableType.txt, DerivedAge.txt.
    #[arg(long)]
    ucd: PathBuf,
    /// Unicode version the files must be, e.g. 17.0.0.
    #[arg(long)]
    unicode: String,
    /// NFC baseline: code points newer than this that NFC can treat
    /// differently across versions are refused.
    #[arg(long)]
    baseline: String,
    /// The verified `sha256sum` file, copied into the provenance header.
    #[arg(long)]
    checksums: PathBuf,
    /// Output file (rustfmt is run on it by regen.sh).
    #[arg(long)]
    out: PathBuf,
}

/// Reason codes. Must match `opaque_string::Reason`'s discriminants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Code {
    Allowed = 0,
    Unassigned = 1,
    Control = 2,
    Ignorable = 3,
    OldHangulJamo = 4,
    JoinControl = 5,
    Exception = 6,
    Disallowed = 7,
    UnstableNormalization = 8,
}

/// RFC 5892 §2.6 Exceptions (F), as RFC 8264 §9.6 includes them.
enum Exception {
    /// PVALID, and CONTEXTO, which this crate allows without its context
    /// rule (passwords are never displayed).
    Allowed,
    Disallowed,
}

fn exception(cp: u32) -> Option<Exception> {
    use Exception::{Allowed, Disallowed};
    Some(match cp {
        // PVALID
        0x00DF | 0x03C2 | 0x06FD | 0x06FE | 0x0F0B | 0x3007 => Allowed,
        // CONTEXTO
        0x00B7 | 0x0375 | 0x05F3 | 0x05F4 | 0x30FB | 0x0660..=0x0669 | 0x06F0..=0x06F9 => Allowed,
        // DISALLOWED
        0x0640 | 0x07FA | 0x302E | 0x302F | 0x3031..=0x3035 | 0x303B => Disallowed,
        _ => return None,
    })
}

fn char_of(cp: u32) -> Option<char> {
    char::from_u32(cp)
}

fn nfkc_changes(cp: u32) -> bool {
    char_of(cp).is_some_and(|c| c.to_string().nfkc().ne(std::iter::once(c)))
}

fn canonical_decomposition(c: char) -> Vec<char> {
    let mut out = Vec::new();
    decompose_canonical(c, |d| out.push(d));
    out
}

/// RFC 8264 §8, FreeformClass, in the order the RFC fixes, with this
/// crate's deviations: CONTEXTJ is refused, CONTEXTO is allowed.
/// `has_compat_decided` counts code points the HasCompat step decided
/// differently from what the later steps would have.
fn derive(ucd: &Ucd, cp: u32, has_compat_decided: &mut Vec<u32>) -> Code {
    let gc = ucd.category(cp);
    if let Some(e) = exception(cp) {
        return match e {
            Exception::Allowed => Code::Allowed,
            Exception::Disallowed => Code::Exception,
        };
    }
    // BackwardCompatible (G) is the empty set (RFC 8264 §9.7).
    if gc == "Cn" && !ucd.noncharacter[cp as usize] {
        return Code::Unassigned;
    }
    if (0x21..=0x7E).contains(&cp) {
        return Code::Allowed;
    }
    if ucd.join_control[cp as usize] {
        return Code::JoinControl;
    }
    if ucd.old_hangul_jamo[cp as usize] {
        return Code::OldHangulJamo;
    }
    if ucd.default_ignorable[cp as usize] || ucd.noncharacter[cp as usize] {
        return Code::Ignorable;
    }
    if gc == "Cc" {
        return Code::Control;
    }
    let later = match gc {
        // LetterDigits (A), OtherLetterDigits (R), Spaces (N), Symbols (O),
        // Punctuation (P): PVALID or FREE_PVAL.
        "Ll" | "Lu" | "Lo" | "Nd" | "Lm" | "Mn" | "Mc" | "Lt" | "Nl" | "No" | "Me" | "Zs"
        | "Sm" | "Sc" | "Sk" | "So" | "Pc" | "Pd" | "Ps" | "Pe" | "Pi" | "Pf" | "Po" => {
            Code::Allowed
        }
        _ => Code::Disallowed,
    };
    // HasCompat (Q) is FREE_PVAL.
    if nfkc_changes(cp) {
        if later != Code::Allowed {
            has_compat_decided.push(cp);
        }
        return Code::Allowed;
    }
    later
}

fn parse_version(s: &str) -> Result<Vec<u8>, Error> {
    s.split('.')
        .map(|p| p.parse::<u8>())
        .collect::<Result<_, _>>()
        .map_err(|_| Error::Invariant(format!("bad version {s:?}")))
}

fn run() -> Result<(), Error> {
    let args = Args::parse();
    let unicode = parse_version(&args.unicode)?;
    let baseline = parse_version(&args.baseline)?;
    let [b_major, b_minor] = baseline[..] else {
        return Err(Error::Invariant("baseline must be MAJOR.MINOR".into()));
    };
    let base = (b_major, b_minor);
    let linked = unicode_normalization::UNICODE_VERSION;
    if unicode != [linked.0, linked.1, linked.2] {
        return Err(Error::Invariant(format!(
            "UCD {} but unicode-normalization has {}.{}.{}",
            args.unicode, linked.0, linked.1, linked.2
        )));
    }
    let checksums = std::fs::read_to_string(&args.checksums).map_err(|source| Error::Read {
        path: args.checksums.clone(),
        source,
    })?;
    let ucd = Ucd::load(&args.ucd, &args.unicode)?;

    // Cross-check the linked normalization tables against the UCD files.
    for cp in 0..CODE_SPACE as u32 {
        let Some(c) = char_of(cp) else { continue };
        let rec = &ucd.records[cp as usize];
        if canonical_combining_class(c) != rec.ccc {
            return Err(Error::Invariant(format!(
                "U+{cp:04X}: ccc {} in UCD, {} linked",
                rec.ccc,
                canonical_combining_class(c)
            )));
        }
        if (0xAC00..=0xD7A3).contains(&cp) {
            continue; // Hangul syllables decompose algorithmically.
        }
        let mut expanded = Vec::new();
        let mut stack: Vec<u32> = vec![cp];
        while let Some(x) = stack.pop() {
            let m = &ucd.records[x as usize].canonical;
            if m.is_empty() {
                expanded.push(x);
            } else {
                stack.extend(m.iter().rev());
            }
        }
        let linked: Vec<u32> = canonical_decomposition(c)
            .into_iter()
            .map(u32::from)
            .collect();
        // The linked decomposition is also canonically reordered.
        let mut a = expanded.clone();
        let mut b = linked.clone();
        a.sort_unstable();
        b.sort_unstable();
        if a != b {
            return Err(Error::Invariant(format!(
                "U+{cp:04X}: decomposition {expanded:X?} in UCD, {linked:X?} linked"
            )));
        }
    }

    // The class.
    let mut has_compat_decided = Vec::new();
    let mut codes: Vec<Code> = (0..CODE_SPACE as u32)
        .map(|cp| derive(&ucd, cp, &mut has_compat_decided))
        .collect();
    if !has_compat_decided.is_empty() {
        return Err(Error::Invariant(format!(
            "HasCompat decided {} code points (expected none): {:X?}",
            has_compat_decided.len(),
            &has_compat_decided[..has_compat_decided.len().min(16)]
        )));
    }

    // The NFC-unstable set S: code points newer than the baseline that an
    // engine without them would normalize differently (non-zero ccc, a
    // canonical decomposition, or a part of a newer code point's
    // decomposition). An engine treats a code point it does not know as a
    // starter with no decomposition, which is right for every other one.
    let newer = |cp: u32| ucd.age[cp as usize] > base;
    let mut component = vec![false; CODE_SPACE];
    for cp in 0..CODE_SPACE as u32 {
        let Some(c) = char_of(cp) else { continue };
        if !newer(cp) {
            continue;
        }
        let d = canonical_decomposition(c);
        if d != [c] {
            for x in d {
                component[u32::from(x) as usize] = true;
            }
        }
        // Stability: a new primary composite must have a new component.
        if let [a, b] = ucd.records[cp as usize].canonical[..]
            && let (Some(ca), Some(cb)) = (char_of(a), char_of(b))
            && compose(ca, cb) == Some(c)
            && !newer(a)
            && !newer(b)
        {
            return Err(Error::Invariant(format!(
                "U+{cp:04X} is a primary composite newer than {} of older parts",
                args.baseline
            )));
        }
    }
    let mut unstable = Vec::new();
    for cp in 0..CODE_SPACE as u32 {
        let Some(c) = char_of(cp) else { continue };
        if codes[cp as usize] != Code::Allowed || !newer(cp) {
            continue;
        }
        if canonical_combining_class(c) != 0
            || canonical_decomposition(c) != [c]
            || component[cp as usize]
        {
            codes[cp as usize] = Code::UnstableNormalization;
            unstable.push(cp);
        }
    }

    // Non-ASCII Zs: the space map's domain.
    let spaces: Vec<u32> = (0x80..CODE_SPACE as u32)
        .filter(|&cp| ucd.category(cp) == "Zs")
        .collect();

    // The canary: the lowest allowed code point of exactly the baseline's
    // age with 0 < ccc < 230. "a", it, U+0301 composes to "á", it on an
    // engine that knows it, and stays as it is on one that does not.
    let canary = (0..CODE_SPACE as u32)
        .filter_map(|cp| char_of(cp).map(|c| (cp, c)))
        .find(|&(cp, c)| {
            ucd.age[cp as usize] == base
                && codes[cp as usize] == Code::Allowed
                && (1..230).contains(&canonical_combining_class(c))
        })
        .map(|(_, c)| c)
        .ok_or_else(|| Error::Invariant("no canary of the baseline's age".into()))?;
    let probe: String = format!("a{canary}\u{0301}").nfc().collect();
    if probe != format!("\u{E1}{canary}") {
        return Err(Error::Invariant(format!(
            "canary U+{:04X} does not compose as expected",
            u32::from(canary)
        )));
    }

    // Runs.
    let mut runs: Vec<(u32, Code)> = Vec::new();
    for (cp, &code) in codes.iter().enumerate() {
        if runs.last().is_none_or(|&(_, last)| last != code) {
            runs.push((cp as u32, code));
        }
    }
    let mut counts: BTreeMap<Code, usize> = BTreeMap::new();
    for cp in 0..CODE_SPACE as u32 {
        if char_of(cp).is_some() {
            *counts.entry(codes[cp as usize]).or_default() += 1;
        }
    }

    let mut out = String::new();
    let w = &mut out;
    let _ = writeln!(w, "//! Generated by `tablegen/regen.sh`. Do not edit.");
    let _ = writeln!(w, "//!");
    let _ = writeln!(
        w,
        "//! RFC 8264 §8 FreeformClass for Unicode {}, with this crate's",
        args.unicode
    );
    let _ = writeln!(
        w,
        "//! deviations: CONTEXTJ refused, CONTEXTO allowed, and code points"
    );
    let _ = writeln!(
        w,
        "//! newer than Unicode {} that NFC treats differently across versions",
        args.baseline
    );
    let _ = writeln!(w, "//! refused.");
    let _ = writeln!(w, "//!");
    let _ = writeln!(w, "//! Inputs (SHA-256):");
    for line in checksums.lines().filter(|l| !l.trim().is_empty()) {
        let _ = writeln!(w, "//!     {line}");
    }
    let _ = writeln!(w, "//!");
    let _ = writeln!(w, "//! Scalar values by outcome:");
    for (code, n) in &counts {
        let _ = writeln!(w, "//!     {code:?}: {n}");
    }
    let _ = writeln!(w, "//!");
    let _ = writeln!(w, "//! UnstableNormalization ({}):", unstable.len());
    for chunk in unstable.chunks(8) {
        let list: Vec<String> = chunk.iter().map(|cp| format!("U+{cp:04X}")).collect();
        let _ = writeln!(w, "//!     {}", list.join(" "));
    }
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "pub(crate) const UNICODE_VERSION: (u8, u8, u8) = ({}, {}, {});",
        unicode[0], unicode[1], unicode[2]
    );
    let _ = writeln!(
        w,
        "pub(crate) const NFC_BASELINE: (u8, u8) = ({b_major}, {b_minor});"
    );
    let _ = writeln!(w);
    let _ = writeln!(w, "/// Non-ASCII General Category Zs.");
    let zs: Vec<String> = spaces
        .iter()
        .map(|cp| format!("'\\u{{{cp:04X}}}'"))
        .collect();
    let _ = writeln!(
        w,
        "pub(crate) const ZS_NON_ASCII: &[char] = &[{}];",
        zs.join(", ")
    );
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "/// A code point of exactly the baseline's age with 0 < ccc < 230."
    );
    let _ = writeln!(
        w,
        "pub(crate) const CANARY: char = '\\u{{{:04X}}}';",
        u32::from(canary)
    );
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "/// `(first code point << 5) | reason code`, ascending; each run lasts"
    );
    let _ = writeln!(w, "/// until the next one starts.");
    let _ = writeln!(w, "pub(crate) const RUNS: &[u32] = &[");
    for (start, code) in &runs {
        let packed = (*start << 5) | *code as u32;
        let _ = writeln!(w, "    0x{packed:07X}, // U+{start:04X} {code:?}");
    }
    let _ = writeln!(w, "];");

    std::fs::write(&args.out, out).map_err(|source| Error::Write {
        path: args.out.clone(),
        source,
    })?;
    eprintln!(
        "{} runs; {} unstable-normalization code points; canary U+{:04X}",
        runs.len(),
        unstable.len(),
        u32::from(canary)
    );
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
