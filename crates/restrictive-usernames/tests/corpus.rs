//! The acceptance corpus against the full policy.
#![cfg(all(feature = "check", feature = "corpus"))]

use restrictive_usernames::corpus::{CHECK, KEY_PAIRS, NORMALIZE, VISUAL_KEY_PAIRS};
use restrictive_usernames::{
    Kind, Policy, Rejected, Rule, check, check_structural, is_valid, key_ci, key_cs, key_visual_ci,
    key_visual_cs, normalize,
};

#[test]
fn check_cases() {
    let mut failures = Vec::new();
    for c in CHECK {
        let got = check(c.input, &Policy::for_kind(c.kind));
        if got != c.rules {
            failures.push(format!(
                "{:?} as {:?}: expected {:?}, got {:?}",
                c.input, c.kind, c.rules, got
            ));
        }
        if is_valid(c.input, &Policy::for_kind(c.kind)) != c.rules.is_empty() {
            failures.push(format!("{:?} as {:?}: is_valid disagrees", c.input, c.kind));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn structural_is_a_subset_of_check() {
    for c in CHECK {
        let policy = Policy::for_kind(c.kind);
        let full = check(c.input, &policy);
        for r in check_structural(c.input, &policy) {
            assert!(
                full.contains(&r),
                "{:?}: structural {r:?} not in {full:?}",
                c.input
            );
        }
    }
}

/// Over ASCII the structural tier decides [`Rule::DisallowedChar`] from a
/// hand-written table; the full check decides it from the Unicode tables.
/// Every ASCII code point, alone, between letters and doubled, under every
/// kind: the structural result stays a subset and the two agree on
/// `DisallowedChar`.
#[test]
fn ascii_structural_decides_disallowed_exactly() {
    let mut failures = Vec::new();
    for kind in [Kind::Username, Kind::Nickname, Kind::Label] {
        let policy = Policy::for_kind(kind);
        for c in '\0'..='\x7f' {
            for input in [c.to_string(), format!("a{c}b"), format!("{c}{c}")] {
                let full = check(&input, &policy);
                let structural = check_structural(&input, &policy);
                if let Some(r) = structural.iter().find(|r| !full.contains(r)) {
                    failures.push(format!(
                        "{input:?} as {kind:?}: structural {r:?} not in {full:?}"
                    ));
                }
                let d = Rule::DisallowedChar;
                if structural.contains(&d) != full.contains(&d) {
                    failures.push(format!(
                        "{input:?} as {kind:?}: structural {structural:?}, full {full:?}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn normalize_cases() {
    let mut failures = Vec::new();
    for c in NORMALIZE {
        let got = normalize(c.input, &Policy::for_kind(c.kind));
        let want = c
            .expected
            .map(str::to_owned)
            .map_err(|r| Rejected(r.to_vec()));
        if got != want {
            failures.push(format!(
                "{:?} as {:?}: expected {want:?}, got {got:?}",
                c.input, c.kind
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn normalize_is_idempotent_and_never_not_nfc() {
    let inputs = CHECK.iter().map(|c| (c.input, c.kind));
    let inputs = inputs.chain(NORMALIZE.iter().map(|c| (c.input, c.kind)));
    for (input, kind) in inputs {
        let policy = Policy::for_kind(kind);
        match normalize(input, &policy) {
            Ok(stored) => {
                assert_eq!(normalize(&stored, &policy), Ok(stored.clone()));
                assert!(check(&stored, &policy).is_empty());
            }
            Err(Rejected(rules)) => {
                assert!(!rules.is_empty());
                assert!(!rules.contains(&Rule::NotNfc), "{input:?}");
            }
        }
    }
}

#[test]
fn key_pairs() {
    let mut failures = Vec::new();
    for p in KEY_PAIRS {
        let ci = key_ci(p.a) == key_ci(p.b);
        let cs = key_cs(p.a) == key_cs(p.b);
        if (ci, cs) != (p.equal_ci, p.equal_cs) {
            failures.push(format!(
                "{:?}/{:?}: expected ci={} cs={}, got ci={ci} cs={cs} (ci {:?}/{:?}, cs {:?}/{:?})",
                p.a,
                p.b,
                p.equal_ci,
                p.equal_cs,
                key_ci(p.a),
                key_ci(p.b),
                key_cs(p.a),
                key_cs(p.b)
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn visual_key_pairs() {
    let mut failures = Vec::new();
    let n = Policy::nickname();
    for p in VISUAL_KEY_PAIRS {
        for name in [p.a, p.b] {
            if let Err(e) = normalize(name, &n) {
                failures.push(format!("{name:?} is not a nickname: {e}"));
            }
        }
        let ci = key_visual_ci(p.a) == key_visual_ci(p.b);
        let cs = key_visual_cs(p.a) == key_visual_cs(p.b);
        if (ci, cs) != (p.equal_ci, p.equal_cs) {
            failures.push(format!(
                "{:?}/{:?}: expected ci={} cs={}, got ci={ci} cs={cs}",
                p.a, p.b, p.equal_ci, p.equal_cs
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Without a right-to-left code point the display order is the stored
/// order, so the display-order keys are the stored-order keys.
#[test]
fn visual_keys_agree_without_rtl() {
    let inputs = CHECK.iter().map(|c| c.input);
    let inputs = inputs.chain(KEY_PAIRS.iter().flat_map(|p| [p.a, p.b]));
    for input in inputs {
        let rtl_or_control = |c: char| {
            ('\u{590}'..='\u{8FF}').contains(&c)
                || ('\u{202A}'..='\u{202E}').contains(&c)
                || ('\u{2066}'..='\u{2069}').contains(&c)
        };
        if input.chars().any(rtl_or_control) {
            continue;
        }
        assert_eq!(key_visual_ci(input), key_ci(input), "{input:?}");
        assert_eq!(key_visual_cs(input), key_cs(input), "{input:?}");
    }
}

#[test]
fn keys_ignore_ingress_spacing() {
    for c in CHECK {
        let padded = format!("\u{3000} {}\u{A0} ", c.input);
        assert_eq!(key_ci(c.input), key_ci(&padded), "{:?}", c.input);
        assert_eq!(key_cs(c.input), key_cs(&padded), "{:?}", c.input);
        assert_eq!(
            key_visual_ci(c.input),
            key_visual_ci(&padded),
            "{:?}",
            c.input
        );
        assert_eq!(
            key_visual_cs(c.input),
            key_visual_cs(&padded),
            "{:?}",
            c.input
        );
    }
}

#[test]
fn highly_restrictive_examples_from_the_spec() {
    let n = Policy::nickname();
    for ok in [
        "ローマ字Latin",
        "한국어Latin",
        "中文Latin",
        "Иван Smith",
        "ㄅㄆ中文Latin",
    ] {
        assert!(!check(ok, &n).contains(&Rule::MixedScript), "{ok}");
    }
    for bad in [
        "\u{410}dmin",
        "\u{3A9}mega",
        "\u{410}dmin Smith",
        "한국어ローマ字",
        "LatinᏣᎳᎩ",
    ] {
        assert!(check(bad, &n).contains(&Rule::MixedScript), "{bad}");
    }
}

#[test]
fn custom_policy_knobs() {
    let p = Policy::nickname().with_max_len(5);
    assert_eq!(check("abcdef", &p), [Rule::Length { min: 1, max: 5 }]);
    let p = Policy::nickname().with_max_mark_run(1);
    assert_eq!(
        check("e\u{323}\u{302}", &p),
        [Rule::NotNfc, Rule::MarkRun { max: 1 }]
    );
    assert_eq!(Policy::for_kind(Kind::Label), Policy::label());
}
