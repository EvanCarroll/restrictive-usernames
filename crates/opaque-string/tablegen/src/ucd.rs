//! Just enough of a Unicode Character Database reader for the derivation.

use std::collections::HashMap;
use std::path::Path;

use crate::error::Error;

/// Code points 0..=0x10FFFF, surrogates included.
pub const CODE_SPACE: usize = 0x11_0000;

/// One record of UnicodeData.txt.
#[derive(Debug, Clone, Default)]
pub struct Record {
    /// General_Category, two letters. Empty for unlisted (unassigned) code
    /// points, which are Cn.
    pub category: String,
    /// Canonical_Combining_Class.
    pub ccc: u8,
    /// The one-level canonical decomposition mapping; empty when there is
    /// none or the mapping is a compatibility one (`<tag> ...`).
    pub canonical: Vec<u32>,
}

/// The properties the RFC 8264 §8 derivation reads, indexed by code point.
pub struct Ucd {
    pub records: Vec<Record>,
    pub noncharacter: Vec<bool>,
    pub join_control: Vec<bool>,
    pub default_ignorable: Vec<bool>,
    /// Hangul_Syllable_Type is L, V or T.
    pub old_hangul_jamo: Vec<bool>,
    /// DerivedAge as (major, minor); (0, 0) for unassigned.
    pub age: Vec<(u8, u8)>,
}

impl Ucd {
    /// General_Category of `cp`, `Cn` when unlisted.
    pub fn category(&self, cp: u32) -> &str {
        match self.records[cp as usize].category.as_str() {
            "" => "Cn",
            gc => gc,
        }
    }

    /// Reads the five files from `dir` and checks each header names
    /// `version`. UnicodeData.txt has no header; regen.sh's checksum covers
    /// it.
    pub fn load(dir: &Path, version: &str) -> Result<Self, Error> {
        let unicode_data = read(dir, "UnicodeData.txt")?;
        let prop_list = read(dir, "PropList.txt")?;
        let core = read(dir, "DerivedCoreProperties.txt")?;
        let hangul = read(dir, "HangulSyllableType.txt")?;
        let age = read(dir, "DerivedAge.txt")?;
        check_header("PropList.txt", &prop_list, "PropList", version)?;
        check_header(
            "DerivedCoreProperties.txt",
            &core,
            "DerivedCoreProperties",
            version,
        )?;
        check_header(
            "HangulSyllableType.txt",
            &hangul,
            "HangulSyllableType",
            version,
        )?;
        check_header("DerivedAge.txt", &age, "DerivedAge", version)?;

        let flag = |text: &str, file: &'static str, want: &str| -> Result<Vec<bool>, Error> {
            let mut out = vec![false; CODE_SPACE];
            for (range, value) in properties(text, file)? {
                if value == want {
                    for cp in range {
                        out[cp as usize] = true;
                    }
                }
            }
            Ok(out)
        };

        let mut old_hangul_jamo = vec![false; CODE_SPACE];
        for (range, value) in properties(&hangul, "HangulSyllableType.txt")? {
            if matches!(value.as_str(), "L" | "V" | "T") {
                for cp in range {
                    old_hangul_jamo[cp as usize] = true;
                }
            }
        }

        let mut ages = vec![(0, 0); CODE_SPACE];
        for (range, value) in properties(&age, "DerivedAge.txt")? {
            let (major, minor) = value.split_once('.').ok_or_else(|| Error::Parse {
                file: "DerivedAge.txt",
                line: 0,
                message: format!("age {value:?}"),
            })?;
            let parsed = (
                major.parse().map_err(|_| bad_age(&value))?,
                minor.parse().map_err(|_| bad_age(&value))?,
            );
            for cp in range {
                ages[cp as usize] = parsed;
            }
        }

        Ok(Self {
            records: unicode_data_records(&unicode_data)?,
            noncharacter: flag(&prop_list, "PropList.txt", "Noncharacter_Code_Point")?,
            join_control: flag(&prop_list, "PropList.txt", "Join_Control")?,
            default_ignorable: flag(
                &core,
                "DerivedCoreProperties.txt",
                "Default_Ignorable_Code_Point",
            )?,
            old_hangul_jamo,
            age: ages,
        })
    }
}

fn bad_age(value: &str) -> Error {
    Error::Parse {
        file: "DerivedAge.txt",
        line: 0,
        message: format!("age {value:?}"),
    }
}

fn read(dir: &Path, name: &str) -> Result<String, Error> {
    let path = dir.join(name);
    std::fs::read_to_string(&path).map_err(|source| Error::Read { path, source })
}

/// The first line of every UCD property file is `# <Name>-<version>.txt`.
fn check_header(file: &'static str, text: &str, name: &str, version: &str) -> Result<(), Error> {
    let first = text.lines().next().unwrap_or_default();
    let expected = format!("# {name}-{version}.txt");
    if first.trim() == expected {
        Ok(())
    } else {
        Err(Error::Version {
            file,
            found: first.trim().to_owned(),
            expected: expected[2..].to_owned(),
        })
    }
}

fn hex(file: &'static str, line: usize, s: &str) -> Result<u32, Error> {
    u32::from_str_radix(s.trim(), 16).map_err(|_| Error::Parse {
        file,
        line,
        message: format!("bad code point {s:?}"),
    })
}

/// `XXXX ; Value # comment` and `XXXX..YYYY ; Value # comment` lines.
fn properties(
    text: &str,
    file: &'static str,
) -> Result<Vec<(std::ops::RangeInclusive<u32>, String)>, Error> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let (cps, value) = line.split_once(';').ok_or_else(|| Error::Parse {
            file,
            line: i + 1,
            message: "no ';'".into(),
        })?;
        let range = match cps.trim().split_once("..") {
            Some((a, b)) => hex(file, i + 1, a)?..=hex(file, i + 1, b)?,
            None => {
                let cp = hex(file, i + 1, cps)?;
                cp..=cp
            }
        };
        out.push((range, value.trim().to_owned()));
    }
    Ok(out)
}

/// UnicodeData.txt, with `<..., First>` / `<..., Last>` range pairs expanded.
fn unicode_data_records(text: &str) -> Result<Vec<Record>, Error> {
    const FILE: &str = "UnicodeData.txt";
    let mut records = vec![Record::default(); CODE_SPACE];
    let mut first: HashMap<String, (u32, Record)> = HashMap::new();
    for (i, line) in text.lines().enumerate() {
        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() != 15 {
            return Err(Error::Parse {
                file: FILE,
                line: i + 1,
                message: format!("{} fields", fields.len()),
            });
        }
        let cp = hex(FILE, i + 1, fields[0])?;
        let ccc = fields[3].parse().map_err(|_| Error::Parse {
            file: FILE,
            line: i + 1,
            message: format!("ccc {:?}", fields[3]),
        })?;
        let canonical = if fields[5].is_empty() || fields[5].starts_with('<') {
            Vec::new()
        } else {
            fields[5]
                .split(' ')
                .map(|s| hex(FILE, i + 1, s))
                .collect::<Result<_, _>>()?
        };
        let record = Record {
            category: fields[2].to_owned(),
            ccc,
            canonical,
        };
        let name = fields[1];
        if let Some(base) = name.strip_suffix(", First>") {
            first.insert(base.to_owned(), (cp, record));
        } else if let Some(base) = name.strip_suffix(", Last>") {
            let (start, rec) = first.remove(base).ok_or_else(|| Error::Parse {
                file: FILE,
                line: i + 1,
                message: format!("{name} without First"),
            })?;
            for c in start..=cp {
                records[c as usize] = rec.clone();
            }
        } else {
            records[cp as usize] = record;
        }
    }
    Ok(records)
}
