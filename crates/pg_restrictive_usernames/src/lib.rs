//! `pg_restrictive_usernames`: the `restrictive-usernames` policy as SQL.
//!
//! Every function is `IMMUTABLE STRICT PARALLEL SAFE`: a pure function of its
//! arguments and the Unicode tables compiled into this library. That is what
//! lets `key_ci` / `key_cs` and `key_visual_ci` / `key_visual_cs` back unique
//! expression indexes, and it is why a table change is a `POLICY_VERSION`
//! bump followed by a `REINDEX`.
//!
//! The functions that run the policy or compute a key declare `COST 50`
//! (the C-function default is 1): each one walks the name through Unicode
//! tables, and the planner should evaluate cheaper quals first.

::pgrx::pg_module_magic!(name, version);

/// The SQL functions. Not a `#[pg_schema]` module: the control file's
/// `schema = restrictive_usernames` makes `CREATE EXTENSION` create that
/// schema and install every object into it (`@extschema@`). A `#[pg_schema]`
/// module of the same name would emit `CREATE SCHEMA IF NOT EXISTS`, which
/// PostgreSQL refuses because the schema already exists and is not a member
/// of the extension.
mod sql {
    use pgrx::prelude::*;
    use restrictive_usernames::{
        Policy, Rule, check, key_ci as ci, key_cs as cs, key_visual_ci as visual_ci,
        key_visual_cs as visual_cs,
    };

    fn names(rules: Vec<Rule>) -> Vec<String> {
        rules.into_iter().map(|r| r.as_str().to_owned()).collect()
    }

    /// `restrictive_usernames.version()`: the crate's `POLICY_VERSION`.
    #[pg_extern(immutable, strict, parallel_safe)]
    fn version() -> &'static str {
        restrictive_usernames::POLICY_VERSION
    }

    /// The Unicode version of every table the policy reads.
    #[pg_extern(immutable, strict, parallel_safe)]
    fn unicode_version() -> &'static str {
        restrictive_usernames::UNICODE_VERSION
    }

    /// Failed rule names for a username in storage form; empty when valid.
    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn username_rules(name: &str) -> Vec<String> {
        names(check(name, &Policy::username()))
    }

    /// Failed rule names for a nickname in storage form; empty when valid.
    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn nickname_rules(name: &str) -> Vec<String> {
        names(check(name, &Policy::nickname()))
    }

    /// Failed rule names for a label in storage form; empty when valid.
    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn label_rules(name: &str) -> Vec<String> {
        names(check(name, &Policy::label()))
    }

    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn is_valid_username(name: &str) -> bool {
        check(name, &Policy::username()).is_empty()
    }

    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn is_valid_nickname(name: &str) -> bool {
        check(name, &Policy::nickname()).is_empty()
    }

    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn is_valid_label(name: &str) -> bool {
        check(name, &Policy::label()).is_empty()
    }

    /// Case-insensitive confusable key (UTS #39 §4 skeleton between two full
    /// case folds, after NFKC).
    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn key_ci(name: &str) -> String {
        ci(name)
    }

    /// As-typed confusable key (UTS #39 §4 skeleton, then full case fold).
    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn key_cs(name: &str) -> String {
        cs(name)
    }

    /// Case-insensitive confusable key of the displayed form (UAX #9 order,
    /// first-strong paragraph direction, as `<bdi>` / `dir="auto"` show it).
    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn key_visual_ci(name: &str) -> String {
        visual_ci(name)
    }

    /// As-typed confusable key of the displayed form.
    #[pg_extern(immutable, strict, parallel_safe, cost = 50)]
    fn key_visual_cs(name: &str) -> String {
        visual_cs(name)
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;
    use restrictive_usernames::corpus::{CHECK, KEY_PAIRS, NORMALIZE, VISUAL_KEY_PAIRS};
    use restrictive_usernames::{Kind, Policy, Rule, normalize};

    fn sql_rules(input: &str, kind: Kind) -> Vec<String> {
        let got = match kind {
            Kind::Username => Spi::get_one_with_args::<Vec<String>>(
                "SELECT restrictive_usernames.username_rules($1)",
                &[input.into()],
            ),
            Kind::Nickname => Spi::get_one_with_args::<Vec<String>>(
                "SELECT restrictive_usernames.nickname_rules($1)",
                &[input.into()],
            ),
            Kind::Label => Spi::get_one_with_args::<Vec<String>>(
                "SELECT restrictive_usernames.label_rules($1)",
                &[input.into()],
            ),
        };
        got.expect("SPI")
            .expect("rules are never NULL for a non-NULL name")
    }

    fn sql_valid(input: &str, kind: Kind) -> bool {
        let got = match kind {
            Kind::Username => Spi::get_one_with_args::<bool>(
                "SELECT restrictive_usernames.is_valid_username($1)",
                &[input.into()],
            ),
            Kind::Nickname => Spi::get_one_with_args::<bool>(
                "SELECT restrictive_usernames.is_valid_nickname($1)",
                &[input.into()],
            ),
            Kind::Label => Spi::get_one_with_args::<bool>(
                "SELECT restrictive_usernames.is_valid_label($1)",
                &[input.into()],
            ),
        };
        got.expect("SPI").expect("never NULL")
    }

    fn names(rules: &[Rule]) -> Vec<String> {
        rules.iter().map(|r| r.as_str().to_owned()).collect()
    }

    fn key(func: &str, input: &str) -> String {
        Spi::get_one_with_args::<String>(
            &format!("SELECT restrictive_usernames.{func}($1)"),
            &[input.into()],
        )
        .expect("SPI")
        .expect("never NULL")
    }

    #[pg_test]
    fn corpus_check_cases() {
        for c in CHECK {
            assert_eq!(
                sql_rules(c.input, c.kind),
                names(c.rules),
                "{:?} as {:?}",
                c.input,
                c.kind
            );
            assert_eq!(
                sql_valid(c.input, c.kind),
                c.rules.is_empty(),
                "{:?}",
                c.input
            );
        }
    }

    /// Rejected ingress cases: their storage form (normalized in Rust) must
    /// fail the same rules in SQL.
    #[pg_test]
    fn corpus_normalize_cases() {
        for c in NORMALIZE {
            let policy = Policy::for_kind(c.kind);
            match (normalize(c.input, &policy), c.expected) {
                (Ok(stored), Ok(want)) => {
                    assert_eq!(stored, want);
                    assert!(sql_rules(&stored, c.kind).is_empty(), "{stored:?}");
                }
                (Err(_), Err(want)) => {
                    let stored = stored_form(c.input, c.kind);
                    assert_eq!(sql_rules(&stored, c.kind), names(want), "{:?}", c.input);
                }
                (got, want) => panic!("{:?}: expected {want:?}, got {got:?}", c.input),
            }
        }
    }

    /// The storage form `normalize` checks: spaces mapped (Nickname, Label),
    /// then NFC. Computed in SQL so the test does not reimplement it.
    fn stored_form(input: &str, kind: Kind) -> String {
        let mapped = match kind {
            Kind::Username => input.to_owned(),
            Kind::Nickname | Kind::Label => Spi::get_one_with_args::<String>(
                "SELECT btrim(regexp_replace($1, '[\u{00A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000} ]+', ' ', 'g'), ' ')",
                &[input.into()],
            )
            .expect("SPI")
            .expect("never NULL"),
        };
        Spi::get_one_with_args::<String>("SELECT normalize($1, NFC)", &[mapped.into()])
            .expect("SPI")
            .expect("never NULL")
    }

    #[pg_test]
    fn corpus_key_pairs() {
        for p in KEY_PAIRS {
            assert_eq!(
                key("key_ci", p.a) == key("key_ci", p.b),
                p.equal_ci,
                "ci {:?}/{:?}",
                p.a,
                p.b
            );
            assert_eq!(
                key("key_cs", p.a) == key("key_cs", p.b),
                p.equal_cs,
                "cs {:?}/{:?}",
                p.a,
                p.b
            );
            assert_eq!(key("key_ci", p.a), restrictive_usernames::key_ci(p.a));
            assert_eq!(key("key_cs", p.a), restrictive_usernames::key_cs(p.a));
        }
    }

    #[pg_test]
    fn corpus_visual_key_pairs() {
        for p in VISUAL_KEY_PAIRS {
            for (f, want) in [("key_visual_ci", p.equal_ci), ("key_visual_cs", p.equal_cs)] {
                assert_eq!(key(f, p.a) == key(f, p.b), want, "{f} {:?}/{:?}", p.a, p.b);
            }
            assert_eq!(
                key("key_visual_ci", p.a),
                restrictive_usernames::key_visual_ci(p.a)
            );
            assert_eq!(
                key("key_visual_cs", p.a),
                restrictive_usernames::key_visual_cs(p.a)
            );
        }
    }

    #[pg_test]
    fn every_function_is_immutable_strict_parallel_safe_in_the_schema() {
        let bad = Spi::get_one::<i64>(
            "SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace \
             WHERE n.nspname = 'restrictive_usernames' \
               AND NOT (p.provolatile = 'i' AND p.proparallel = 's' AND p.proisstrict \
                        AND p.pronargs = (p.proname NOT IN ('version', 'unicode_version'))::int \
                        AND p.procost = CASE WHEN p.proname IN ('version', 'unicode_version') \
                                             THEN 1 ELSE 50 END)",
        )
        .expect("SPI")
        .expect("count");
        assert_eq!(bad, 0);
        let names = Spi::get_one::<Vec<String>>(
            "SELECT array_agg(p.proname::text ORDER BY p.proname) FROM pg_proc p \
             JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = 'restrictive_usernames'",
        )
        .expect("SPI")
        .expect("functions");
        assert_eq!(
            names,
            [
                "is_valid_label",
                "is_valid_nickname",
                "is_valid_username",
                "key_ci",
                "key_cs",
                "key_visual_ci",
                "key_visual_cs",
                "label_rules",
                "nickname_rules",
                "unicode_version",
                "username_rules",
                "version",
            ]
        );
        for f in ["key_ci", "key_cs", "key_visual_ci", "key_visual_cs"] {
            let v = Spi::get_one::<String>(&format!(
                "SELECT provolatile::text FROM pg_proc WHERE oid = 'restrictive_usernames.{f}(text)'::regprocedure"
            ))
            .expect("SPI")
            .expect("provolatile");
            assert_eq!(v, "i", "{f}");
        }
    }

    #[pg_test]
    fn defaults_and_versions() {
        assert_eq!(
            Spi::get_one::<Vec<String>>("SELECT restrictive_usernames.nickname_rules('id:x')")
                .expect("SPI")
                .expect("rules"),
            Vec::<String>::new()
        );
        assert_eq!(
            Spi::get_one::<String>("SELECT restrictive_usernames.version()").expect("SPI"),
            Some(restrictive_usernames::POLICY_VERSION.to_owned())
        );
        assert_eq!(
            Spi::get_one::<String>("SELECT restrictive_usernames.unicode_version()").expect("SPI"),
            Some(restrictive_usernames::UNICODE_VERSION.to_owned())
        );
        assert_eq!(
            Spi::get_one::<bool>("SELECT restrictive_usernames.key_ci(NULL) IS NULL").expect("SPI"),
            Some(true)
        );
    }

    /// Trusted, so a database owner installs it without a superuser. The
    /// installed control file is what CREATE EXTENSION reads; check that,
    /// not the source.
    #[pg_test]
    fn installed_version_is_trusted() {
        assert_eq!(
            Spi::get_one::<bool>(
                "SELECT v.superuser AND v.trusted FROM pg_available_extension_versions v \
                 JOIN pg_extension e ON e.extname = v.name AND e.extversion = v.version \
                 WHERE v.name = 'pg_restrictive_usernames'"
            )
            .expect("SPI"),
            Some(true)
        );
    }

    #[pg_test]
    fn key_ci_costs_50() {
        assert_eq!(
            Spi::get_one::<f32>("SELECT procost FROM pg_proc WHERE proname = 'key_ci'")
                .expect("SPI"),
            Some(50.0)
        );
    }

    /// STRICT: a NULL name gives NULL, which a domain CHECK admits; NOT NULL
    /// on the column is what refuses a missing name.
    #[pg_test]
    fn null_name_gives_null() {
        for f in [
            "username_rules",
            "nickname_rules",
            "is_valid_username",
            "is_valid_nickname",
            "label_rules",
            "is_valid_label",
            "key_ci",
            "key_cs",
            "key_visual_ci",
            "key_visual_cs",
        ] {
            let is_null =
                Spi::get_one::<bool>(&format!("SELECT restrictive_usernames.{f}(NULL) IS NULL"))
                    .expect("SPI");
            assert_eq!(is_null, Some(true), "{f}(NULL)");
        }
    }

    #[pg_test]
    fn domain_check_refuses_invalid_and_admits_null() {
        Spi::run(
            "CREATE DOMAIN nick AS text \
                 CHECK (restrictive_usernames.is_valid_nickname(VALUE)); \
             CREATE TABLE n (v nick); \
             INSERT INTO n VALUES ('Mountain Host'), (NULL);",
        )
        .expect("SPI");
    }

    #[pg_test(error = "value for domain nick violates check constraint \"nick_check\"")]
    fn domain_check_refuses_a_hash() {
        Spi::run(
            "CREATE DOMAIN nick AS text \
                 CHECK (restrictive_usernames.is_valid_nickname(VALUE)); \
             SELECT '#x'::nick;",
        )
        .expect("SPI");
    }

    #[pg_test(error = "value for domain nick violates check constraint \"nick_check\"")]
    fn domain_check_refuses_a_bare_dot() {
        Spi::run(
            "CREATE DOMAIN nick AS text \
                 CHECK (restrictive_usernames.is_valid_nickname(VALUE)); \
             SELECT '.'::nick;",
        )
        .expect("SPI");
    }

    #[pg_test]
    fn unique_index_on_key_ci_is_accepted() {
        Spi::run(
            "CREATE TABLE t (c text); \
             CREATE UNIQUE INDEX t_ci ON t (restrictive_usernames.key_ci(c)); \
             INSERT INTO t VALUES ('paypal');",
        )
        .expect("SPI");
    }

    #[pg_test(error = "duplicate key value violates unique constraint \"t_ci\"")]
    fn unique_index_on_key_ci_refuses_a_confusable() {
        Spi::run(
            "CREATE TABLE t (c text); \
             CREATE UNIQUE INDEX t_ci ON t (restrictive_usernames.key_ci(c)); \
             INSERT INTO t VALUES ('paypal'); \
             INSERT INTO t VALUES ('p\u{430}ypal');",
        )
        .expect("SPI");
    }

    #[pg_test(error = "duplicate key value violates unique constraint \"t_vci\"")]
    fn unique_index_on_key_visual_ci_refuses_a_reordering() {
        Spi::run(
            "CREATE TABLE t (c text); \
             CREATE UNIQUE INDEX t_vci ON t (restrictive_usernames.key_visual_ci(c)); \
             INSERT INTO t VALUES ('Dan \u{5D0}\u{5D1}\u{5D9}'); \
             INSERT INTO t VALUES ('\u{5D0}\u{5D1}\u{5D9} Dan');",
        )
        .expect("SPI");
    }
}

/// Required by `cargo pgrx test`.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
