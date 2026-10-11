//! Differential: this crate against precis-profiles' OpaqueString, an
//! independent implementation whose tables are Unicode 6.3.0.
#![cfg(feature = "nfc-tables")]

use precis_profiles::OpaqueString;
use precis_profiles::precis_core::profile::PrecisFastInvocation;
use precis_profiles::precis_core::{DerivedPropertyValue, Error};

/// CONTEXTO code points precis-profiles refuses without their context and
/// this crate allows anywhere (CONFORMANCE.md §8). The Arabic-Indic digits
/// are CONTEXTO too, but their rule only refuses mixing the two sets, so a
/// lone digit agrees.
const CONTEXTO_ALONE: &[char] = &['\u{00B7}', '\u{0375}', '\u{05F3}', '\u{05F4}', '\u{30FB}'];

/// Every scalar value assigned in Unicode 6.3: both accept or both refuse,
/// and accepted ones prepare to the same string.
#[test]
fn every_code_point_assigned_in_unicode_6_3() {
    let mut failures = Vec::new();
    for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
        let s = c.to_string();
        let theirs = OpaqueString::enforce(s.as_str());
        let unassigned_in_6_3 = matches!(
            &theirs,
            Err(Error::BadCodepoint(info)) if matches!(info.property, DerivedPropertyValue::Unassigned)
        );
        if unassigned_in_6_3 {
            continue;
        }
        let ours = opaque_string::enforce(&s);
        let agree = match (&ours, &theirs) {
            (Ok(a), Ok(b)) => a.as_str() == b.as_ref(),
            (Err(_), Err(_)) => true,
            _ => CONTEXTO_ALONE.contains(&c) && ours.is_ok(),
        };
        if !agree {
            failures.push(format!(
                "U+{:04X}: ours {ours:?}, theirs {theirs:?}",
                u32::from(c)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} disagreements:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The documented multi-code-point deviations, held against the RFC's
/// behaviour as precis-profiles implements it.
#[test]
fn documented_deviations() {
    // CONTEXTJ: ZWJ after a virama is valid in the RFC, refused here.
    let zwj = "\u{0915}\u{094D}\u{200D}";
    assert!(OpaqueString::enforce(zwj).is_ok());
    assert!(opaque_string::enforce(zwj).is_err());
    // CONTEXTO: mixed Arabic-Indic digit sets are refused by the RFC.
    let digits = "\u{0661}\u{06F2}";
    assert!(OpaqueString::enforce(digits).is_err());
    assert!(opaque_string::enforce(digits).is_ok());
    // The class is checked on the prepared form: jamo that compose to a
    // modern syllable are refused by the RFC's preparation, accepted here.
    let jamo = "\u{1100}\u{1161}";
    assert!(OpaqueString::enforce(jamo).is_err());
    assert_eq!(opaque_string::enforce(jamo).unwrap().as_str(), "\u{AC00}");
}
