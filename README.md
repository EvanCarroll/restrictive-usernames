# restrictive-usernames

A Unicode policy for usernames, nicknames and labels, as a Rust library
(`restrictive-usernames`) and a PostgreSQL extension
(`pg_restrictive_usernames`) built from the same code. The database and the
application call the same function, so there is one implementation of the
rules, not two kept in step by hand.

* **Identifier profile** (UTS #39 §3.1): letters, digits, marks and a short
  list of punctuation from Recommended, Inclusion and Limited_Use scripts.
  No emoji, no symbols, no invisibles, no compatibility look-alikes (`ﬁ`,
  fullwidth, `𝓑𝓸𝓫`).
* **Highly Restrictive** script mixing (UTS #39 §5.2), per word: `Аdmin` with
  a Cyrillic `А` is refused, `ローマ字Latin` and `Иван Smith` are accepted.
* **Marks**: no Zalgo stacks, no repeated mark, no mark without a base.
* **Confusable keys** (UTS #39 §4 skeletons) for unique expression indexes:
  `paypal` and `pаypal` cannot both exist. Display-order variants for
  nicknames: `Dan אבי` and `אבי Dan` display alike, so they cannot both
  exist either.

Policy is parameters: kind, length bounds and mark-run cap. Nothing here is
specific to one application, except that the rule vocabulary reserves
`not_taken` for the application's own availability check, so a client
checklist and the server share one enum. An application that needs an escape syntax (a
sigil that marks "this is an id, not a name") picks a character the kind's
profile refuses, e.g. `#` for nicknames, instead of reserving a prefix.

The kinds follow the shape of the PRECIS profiles but are not conformant
implementations of them. [CONFORMANCE.md](CONFORMANCE.md) lists every
difference from RFC 8264/8265/8266, UTS #39 and UAX #9: whether it is
stricter or more liberal, and why.

## The three kinds

| Kind | Shape | Allowed | Length (default) | Script rule |
|---|---|---|---|---|
| `username` | PRECIS IdentifierClass (RFC 8264 §4.2, RFC 8265) | Identifier_Type Recommended / Limited_Use, XID_Continue: exactly letters, marks, digits and `_`; first code point a letter (L*); no whitespace | 3 to 32 code points | Highly Restrictive |
| `nickname` | PRECIS Nickname (RFC 8266) | the identifier profile plus its Inclusion punctuation (`' - . :`, U+00B7, U+0375, U+058A, U+05F3, U+05F4, U+06FD, U+06FE, U+0F0B, U+2010, U+2019, U+2027, U+30A0, U+30FB); at least one letter or digit (L* or N*); words joined by single U+0020 | 1 to 32 | Highly Restrictive, per word |
| `label` | free text (room, scene, avatar names) | anything but Identifier_Type Not_Character (incl. unassigned and private use), Default_Ignorable, Deprecated, and controls; symbols and emoji are fine, but VS16, ZWJ and tag sequences are Default_Ignorable and so refused | 1 to 50 | none |

All kinds: stored in NFC; marks are General Category M* counted on the NFD
form (so spacing-looking vowel signs with combining class 0, such as
Devanagari U+0941 or Thai U+0E34, count): at most 4 in a row (exactly 4
distinct marks pass, so `á` plus three other accents is legal), never the
same mark twice in a row, and every word starts with a non-mark.

## Rules

Checks report every failed rule (never short-circuit), in this order, by the
names below. Those names are the serde names of the Rust `Rule` enum and the
elements of the SQL `text[]`.

| Name | Meaning |
|---|---|
| `empty` | empty after trimming White_Space (reported alone) |
| `length` | code-point count outside `[min, max]` |
| `first_not_letter` | username: first code point is not a letter |
| `whitespace` | username: any White_Space; nickname/label: a space other than U+0020, or a run of spaces |
| `edge_whitespace` | nickname/label: leading or trailing U+0020 |
| `control_char` | a General Category Cc code point |
| `disallowed_char` | a code point outside the kind's profile; also a nickname with no letter or digit (`.`, `...`, `_`) |
| `not_nfc` | not in NFC (storage form only; `normalize` never returns it) |
| `mixed_script` | a word is not Highly Restrictive |
| `mark_run` | more than `max_mark_run` marks in a row, or the same mark twice in a row |
| `no_base` | a word that starts with a mark (or is only marks) |
| `not_taken` | never produced here; an application rule kept in the vocabulary so client checklists share one enum |

## SQL API

`CREATE EXTENSION pg_restrictive_usernames;` creates schema
`restrictive_usernames`. The extension is **trusted** (every function is
pure and `IMMUTABLE`, with no file, network or catalog access), so any role
with `CREATE` on the database can install it, not only a superuser. The
script still runs as the bootstrap superuser, which `LANGUAGE c` functions
require: the extension and the schema belong to the installing role, the
functions to the bootstrap superuser. Every function is `IMMUTABLE STRICT PARALLEL SAFE`,
and every function but the two version functions declares `COST 50` (the
C-function default is 1): each walks the name through Unicode tables, so the
planner should evaluate cheaper quals first. The `*_rules` and `is_valid_*`
functions check the **storage form** (NFC, spaces already mapped); the
application normalizes on ingress.

```sql
restrictive_usernames.version() RETURNS text            -- POLICY_VERSION
restrictive_usernames.unicode_version() RETURNS text    -- every table's Unicode version
restrictive_usernames.username_rules(name text) RETURNS text[]
restrictive_usernames.nickname_rules(name text) RETURNS text[]
restrictive_usernames.label_rules(name text) RETURNS text[]
restrictive_usernames.is_valid_username(name text) RETURNS boolean
restrictive_usernames.is_valid_nickname(name text) RETURNS boolean
restrictive_usernames.is_valid_label(name text) RETURNS boolean
restrictive_usernames.key_ci(text) RETURNS text         -- case-insensitive confusable key
restrictive_usernames.key_cs(text) RETURNS text         -- as-typed confusable key
restrictive_usernames.key_visual_ci(text) RETURNS text  -- key_ci of the displayed form
restrictive_usernames.key_visual_cs(text) RETURNS text  -- key_cs of the displayed form
```

NULL handling: every function is `STRICT`, so a NULL `name` gives NULL (a
domain CHECK then passes, as for any nullable domain; use `NOT NULL` where
needed).

Typical use:

```sql
CREATE DOMAIN nickname AS text
    CHECK (restrictive_usernames.is_valid_nickname(VALUE));

-- usernames: one word, one script, so stored order is display order
CREATE UNIQUE INDEX uq_users_username_ci ON users (restrictive_usernames.key_ci(username));
CREATE UNIQUE INDEX uq_users_username_cs ON users (restrictive_usernames.key_cs(username));

-- nicknames: words of either direction, so key the displayed form
CREATE UNIQUE INDEX uq_users_name_vci ON users (restrictive_usernames.key_visual_ci(name));
CREATE UNIQUE INDEX uq_users_name_vcs ON users (restrictive_usernames.key_visual_cs(name));
```

A name is available when neither index on its column objects. Lookups must
use the indexed expression (`key_visual_ci(name) = key_visual_ci($1)` on a
nickname column) or the planner cannot use the index. Applications using roles
other than the schema owner (the role that installed the extension) need
`GRANT USAGE ON SCHEMA restrictive_usernames`, or every insert fails inside
the domain check.

### Keys

* `key_ci(x) = fold(skeleton(fold(nfkc(prep(x)))))`
* `key_cs(x) = fold(skeleton(nfkc(prep(x))))`

`prep` maps the Unicode spaces to U+0020, collapses runs and trims (RFC 8266
§2.2), so lookup text keys like its storage form. `fold` is full case
folding (C+F, `ß` → `ss`). `skeleton` is UTS #39 §4. Neither order alone
catches everything, hence two keys:

| Pair | `key_ci` | `key_cs` |
|---|---|---|
| `ADMIN` / `admin` | equal | differ |
| `Iris` / `lris` | differ | equal |
| `Bob` / `B0b`, `pаypal` / `paypal`, `ﬁsh` / `fish`, `Ｆｕｌｌ` / `Full`, `Straße` / `Strasse` | equal | equal |
| `Jörg` / `Jorg`, `İstanbul` / `istanbul`, `vv` / `w`, `cl` / `d` | differ | differ |

### Display-order keys (nicknames)

* `key_visual_ci(x) = fold(skeleton(fold(visual(nfkc(prep(x))))))`
* `key_visual_cs(x) = fold(skeleton(visual(nfkc(prep(x)))))`

Text is stored in logical order and displayed in the order the Unicode
Bidirectional Algorithm (UAX #9) computes. Once a nickname mixes
right-to-left words (Hebrew, Arabic) with Latin words or digits, two
different stored strings can display identically, and keys over the stored
order cannot see it. `visual` is the UAX #9 display order with the paragraph
direction from the first strong code point (rules P2, P3), so the keys
compare what the reader sees:

| Pair | Displayed | `key_ci` / `key_cs` | `key_visual_ci` / `key_visual_cs` |
|---|---|---|---|
| `Dan אבי` / `אבי Dan` | both `Dan יבא` | differ | equal |
| `2 Dan א-1` / `2 Dan 1-א` | both `2 Dan 1-א` | differ | equal |
| `ADMIN אבי` / `אבי admin` | `ADMIN יבא` / `admin יבא` | differ | ci equal, cs differ |
| `ADMIN ٢ וו` / `ADMIN ۲ וו` | `ADMIN וו ٢` / `ADMIN ۲ וו` | equal | differ |
| `Dan Jr. אבי` / `אבי Dan Jr.` | `Dan Jr. יבא` / `.Dan Jr יבא` | differ | differ |
| `John Smith` / `Smith John` | as stored | differ | differ |

The fourth row is the reason the display-order keys replace the stored-order
keys on a nickname column rather than joining them: confusable digits of
different Bidi_Class land on opposite sides of the Hebrew word, the names
look different, and only the stored-order keys would refuse the second.
Without a right-to-left code point (Bidi_Class R, AL or AN, or an explicit
right-to-left or first-strong embedding, override or isolate) display order
is stored order, so the display-order keys equal `key_ci` / `key_cs`
exactly, at the same cost. Both are still needed, for the reason above.

Usernames keep `key_ci` / `key_cs`: a username is one word in one script, so
its display order is fixed.

#### Rendering contract

The display-order keys model one rendering: the name isolated from the text
around it, with its direction from its first strong code point. Every
surface that shows a nickname must render it that way:

* HTML: `<bdi>name</bdi>`, or `dir="auto"` on the element holding the name
  (CSS `unicode-bidi: isolate` with `direction` from the content).
* Plain text (email, notifications, logs, exports): U+2068 FIRST STRONG
  ISOLATE, the name, U+2069 POP DIRECTIONAL ISOLATE.

A surface that renders a name without isolation can show it differently
(`1 א` reads `1 א` inside English text, `א 1` under `dir="auto"`), and
scrambles the text around it besides (`שלום 5 points` reads `5 םולש points`).

## Rust API

```toml
restrictive-usernames = "0.5"
# browser / wasm: vocabulary and structural checks only (serde + thiserror)
restrictive-usernames = { version = "0.5", default-features = false }
```

Minimum Rust: 1.85. API documentation: [docs.rs](https://docs.rs/restrictive-usernames).

* `check(input, &policy) -> Vec<Rule>`, `is_valid`, `normalize(input,
  &policy) -> Result<String, Rejected>` (ingress: space mapping for
  nicknames and labels, NFC, then `check`), `key_ci`, `key_cs`,
  `key_visual_ci`, `key_visual_cs`: feature `check` (default).
* `check_structural(input, &policy)`: always available; the rules `core`
  can decide without Unicode tables (empty, length, first letter,
  whitespace, edge whitespace, controls) plus `disallowed_char` as far as
  ASCII decides it: for a username an ASCII code point other than
  `[A-Za-z0-9_]` (`user-name`, `a.b`); for a nickname the same but also
  admitting `' - . :` (`#x` refused, `O'Brien`, `St. John`, `id:x` fine),
  and an all-ASCII nickname with no letter or digit (`...`). Labels get no
  `disallowed_char` here. Always a subset of `check`; over ASCII it decides
  `disallowed_char` exactly as `check` does (an exhaustive test holds every
  ASCII code point to that).
* `Policy::username()`, `::nickname()`, `::label()`, `Policy::for_kind(k)`,
  then `.with_min_len(n)`, `.with_max_len(n)`, `.with_max_mark_run(n)`, or
  assign the fields. `Policy` is `#[non_exhaustive]`, so there is no struct
  literal outside the crate.
* `Policy::parse_rule(name)` turns a SQL rule name back into a `Rule` with
  the `length` / `mark_run` parameters filled from the policy. `str::parse`
  returns those two with zeroed parameters.
* Feature `corpus`: the acceptance corpus as `pub const` tables, used by both
  test suites.

## JavaScript / npm

The library is also on npm as WebAssembly (wasm-bindgen), built from
`crates/restrictive-usernames-wasm`:

* `restrictive-usernames`: `checkStructural`, `defaultPolicy`, `ruleNames`,
  `policyVersion`. No Unicode tables (about 19 KB of gzipped wasm), for
  instant feedback in a browser. Always a subset of `check`.
* `restrictive-usernames-full`: adds `check`, `isValid`, `normalize`
  (`{ ok: true, value } | { ok: false, rules }`), `keyCi`, `keyCs`,
  `keyVisualCi`, `keyVisualCs` and `unicodeVersion` (about 190 KB), for a
  JavaScript server that must compute the same keys as the database.

Kinds and rules are their serde names, and an unknown kind throws. The
optional `minLen` / `maxLen` arguments override the kind's length bounds.
Both packages are ES modules built with `--target web`:

```js
import init, { checkStructural } from "restrictive-usernames";
await init();                         // Node: initSync({ module: bytes })
checkStructural("a.b", "username");   // ["disallowed_char"]
```

`policyVersion()` equals `restrictive_usernames.version()` of a database on
the same release. Keys computed under another policy version may not match
the stored ones.

Build: `crates/restrictive-usernames-wasm/build-npm.sh --test --pack`
(needs the wasm32 target, a wasm-bindgen CLI equal to the crate's version,
wasm-opt, node and npm). The packages land in `target/npm/`.

## Passwords: `opaque-string`

`crates/opaque-string` is a separate crate, versioned on its own, that
implements RFC 8265 §4.2 OpaqueString, the PRECIS profile for passwords.
It makes the same password always become the same bytes before OPAQUE
(RFC 9807) or a password hash: `enforce` at registration, `prepare` at
login. The npm package `opaque-string` is the same code using the
browser's own NFC (about 16 KB gzipped).

It is not part of the PostgreSQL extension and never will be, because
passwords must not reach SQL. See the
[crate README](crates/opaque-string/README.md),
[CONFORMANCE.md §8](CONFORMANCE.md#8-opaquestring-opaque-string-crate)
and [PASSWORDS.md](PASSWORDS.md).

## Unicode versions

| Table | Crate | Unicode |
|---|---|---|
| Identifier_Type, confusables (UTS #39) | `unicode-security` 0.1 | 16.0.0 |
| NFC / NFD / NFKC, canonical combining class | `unicode-normalization` 0.1 | 17.0.0 |
| Script_Extensions | `unicode-script` 0.5 | 17.0.0 |
| XID_Start / XID_Continue | `unicode-ident` 1 | 18.0.0 |
| Case folding (C+F) | `caseless` 0.2 | 16.0.0 |
| Bidi_Class, UAX #9 reordering | `unicode-bidi` 0.3 | 16.0.0 |

`restrictive_usernames.unicode_version()` reports the same. The six crates
are pinned to exact versions (`=0.1.25`, ...), because a semver-compatible
release that moved a table would let an application and the database
compute different keys under one `POLICY_VERSION`. A compile-time assertion
holds each crate's own version constant to the table above, so linking
other tables (a `[patch]`, a fork) fails to compile, here and in every
crate that depends on this one; a unit test pins `UNICODE_VERSION` to the
same constants, so moving a table needs a `POLICY_VERSION` bump.

## Upgrades

The keys are only as stable as the rules and Unicode tables that compute
them. Any change to either bumps `POLICY_VERSION`, and every database must
then load the new library, resolve the rows the new policy rejects or makes
collide, and `REINDEX` every index on the keys before consumers built
against the new crate start. [UPGRADING.md](UPGRADING.md) has the
procedure and the notes for each release; [CHANGELOG.md](CHANGELOG.md)
says what changed.

A consumer can refuse to start against a database that has not been
upgraded:

```rust
/// `db` is `SELECT restrictive_usernames.version()`.
fn require_policy(db: &str) {
    assert_eq!(db, restrictive_usernames::POLICY_VERSION, "database policy differs: UPGRADING.md");
}
```

## Without the extension (managed PostgreSQL)

Hosted PostgreSQL (Amazon RDS and Aurora, Cloud SQL, Azure, Supabase,
Neon and most others) cannot load a custom shared library. The policy then
lives in the application, and the database holds the keys as plain
columns:

```sql
CREATE TABLE users (
    id       bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    username text NOT NULL,
    -- computed by the application: restrictive_usernames::key_ci / key_cs
    username_key_ci text NOT NULL,
    username_key_cs text NOT NULL,
    -- the policy version the keys were computed under
    username_policy text NOT NULL,
    CONSTRAINT uq_users_username_key_ci UNIQUE (username_key_ci),
    CONSTRAINT uq_users_username_key_cs UNIQUE (username_key_cs),
    CONSTRAINT chk_users_username_length
        CHECK (char_length(username) BETWEEN 3 AND 32)
);
```

The application runs `normalize` on ingress, writes the name with its keys
and `POLICY_VERSION`, and looks names up by key. What this gives up:

* **One writer.** The database cannot check a name or compute a key, so
  every path that writes the table (other services, migrations, `psql`)
  must go through the library, or a row can bypass the policy. Revoke
  direct `INSERT` / `UPDATE` from everyone else.
* **Upgrades rewrite rows instead of rebuilding indexes.** A policy bump
  means recomputing every key column (find the stale rows by
  `username_policy`), resolving the collisions the `UPDATE` reports, then
  switching consumers. Under the extension, the same step is a `REINDEX`.

With the extension the keys are expression indexes and a domain `CHECK`
guards every writer, in any language, so prefer it wherever you control
the server.

## Known limitations

* **Display-order keys** model one rendering (first-strong isolation, see
  "Rendering contract"); a surface that renders differently can show names
  alike that the keys keep apart. They do not apply Bidi_Mirrored glyph
  substitution (UAX #9 L4): the nickname profile has no mirrored code
  points, so this matters only for keys taken over other text (brackets in
  a right-to-left run are drawn flipped, and the key does not flip them).

* **ZWNJ / ZWJ in words** (Persian `علی‌رضا`): both are Default_Ignorable
  and refused. Allowing them needs the RFC 5892 Appendix A CONTEXTJ rules.
* **Digit numbering systems** (UTS #39 §5.3 mixed-number detection) is not
  implemented. In practice the identifier profile and the script rule cover
  most of it: fullwidth and mathematical digits are Not_NFKC, and native
  digits carry their script, so `Bob٢` (Arabic-Indic) is mixed script.
  Two Common-script digit sets in one word are not detected.
* `unicode-security` reports one Identifier_Type per code point, while the
  property is multi-valued since Unicode 15.1 (`Limited_Use Not_XID`). The
  profile therefore also requires XID_Continue for Recommended and
  Limited_Use code points, which restores the Not_XID half (e.g. Javanese
  `꧁꧂` stay refused).
* The username first-letter rule is Alphabetic ∧ XID_Start ∧ ¬N*, which over
  the username profile is exactly General Category L* (measured across all
  code points). The std-only structural tier uses Alphabetic ∧ ¬N*.
* Restriction level is computed on `AugmentedScriptSet` directly rather than
  with `unicode-security`'s `detect_restriction_level`, which returns
  Unrestricted for any Limited_Use code point and would refuse Cherokee,
  Canadian Syllabics, Mongolian, Tifinagh, Yi, N'Ko and others.

## Build and test

Library (host):

```sh
cargo test --all-features
cargo test --no-default-features
cargo clippy --all-targets --all-features
rustup target add wasm32-unknown-unknown
cargo build -p restrictive-usernames --no-default-features --target wasm32-unknown-unknown
cargo test -p restrictive-usernames-wasm --all-features    # the npm bindings, on the host
cargo +1.85 test -p restrictive-usernames --all-features    # minimum supported Rust
```

The workspace's default member is the library, so these never touch the
pgrx crate, which needs PostgreSQL server headers and `cargo pgrx init`.

### Fuzzing

`crates/restrictive-usernames/fuzz` is a `cargo-fuzz` crate, outside the
workspace, with three targets. Each one asserts properties, not just "does
not panic":

| Target | Asserts |
|---|---|
| `check` | `is_valid` agrees with `check`; rules come in `Rule::ALL` order without duplicates; `empty` is reported alone and exactly for blank input; `length` matches the policy; `Policy::parse_rule` restores every rule from its SQL name; `check_structural` is a subset of `check` and decides `disallowed_char` identically on ASCII. Policies are fuzzed too (kind, `min_len`, `max_len`, `max_mark_run`). |
| `normalize` | an accepted name's storage form is NFC, passes `check`, is a fixed point of `normalize`, and has the same four keys as the input; a refusal is non-empty, ordered, and never `not_nfc` |
| `keys` | all four keys are unchanged by NFC, NFD, edge spaces, and U+0020 written as another mapped space |

The properties live in `fuzz/src/lib.rs`, so they also run on stable over
the acceptance corpus (`cargo test` in `fuzz/`, which CI runs).

```sh
cd crates/restrictive-usernames
(cd fuzz && cargo test -- --ignored write_seed_corpus)   # seed fuzz/corpus/ from the acceptance corpus
cargo +nightly fuzz run check -- -dict=fuzz/unicode.dict -max_len=256
cargo +nightly fuzz run normalize -- -dict=fuzz/unicode.dict -max_len=256
cargo +nightly fuzz run keys -- -dict=fuzz/unicode.dict -max_len=256
```

`fuzz/unicode.dict` lists the code points the policy treats specially:
mapped spaces, invisibles, combining marks, right-to-left letters and
digits, contextual punctuation, confusables, compatibility characters and
Hangul jamo. Without it, libFuzzer rarely produces valid multi-byte UTF-8.
A failure leaves its input in `fuzz/artifacts/<target>/`. Reproduce it with
`cargo +nightly fuzz run <target> <file>`, then add the name to the
acceptance corpus (`src/corpus.rs`) once it is fixed. CI runs each target
for 60 seconds on every push and uploads `fuzz/artifacts/` on failure.

`crates/opaque-string/fuzz` does the same for the password crate, with
targets `prepare` and `enforce`, dictionary `fuzz/opaque.dict`, and seeds
from `crates/opaque-string/tests/corpus.json`. Its properties:
- `prepare` is idempotent and returns NFC with no non-ASCII spaces;
- `enforce(prepare(x)) == enforce(x)`;
- an accepted password is a fixed point;
- the first refused code point and the count are exact;
- errors never contain the password.

### Extension

`container/build-extension.sh` builds with buildah (no Containerfile):

```sh
container/build-extension.sh --test            # cargo pgrx test pg18, then package
container/build-extension.sh --out ./out       # also copy the packaged tree out
PG_MAJOR=14 container/build-extension.sh       # another major, 14 to 18
```

The script builds in a throwaway `docker.io/library/rust:1-trixie` container
with PostgreSQL from apt.postgresql.org, installs `cargo-pgrx` at exactly the
resolved `pgrx` version, and runs the tests as an unprivileged user. It
builds for the host architecture only: pgrx needs the target's server
headers, and `cargo pgrx test` runs a cluster. Options: `IMAGE_NAME`
(default `localhost/pg-restrictive-usernames`), `TAG_SUFFIX` (e.g. `-arm64`,
for CI to join the per-architecture tags into one manifest), `IMAGE_SOURCE`
and `IMAGE_REVISION`. It commits three images:

* **`:pg18`**, `FROM scratch`, files under one prefix, owned by root, for a
  database image build to `buildah copy --from`:
  ```
  /opt/pg-restrictive-usernames/pg18/usr/lib/postgresql/18/lib/pg_restrictive_usernames.so
  /opt/pg-restrictive-usernames/pg18/usr/share/postgresql/18/extension/pg_restrictive_usernames.control
  /opt/pg-restrictive-usernames/pg18/usr/share/postgresql/18/extension/pg_restrictive_usernames--<version>.sql
  /opt/pg-restrictive-usernames/pg18/usr/share/postgresql/18/extension/pg_restrictive_usernames--<old>--<new>.sql   (upgrade scripts)
  /opt/pg-restrictive-usernames/pg18/schema.sql   (with --test: the generated schema)
  /opt/pg-restrictive-usernames/pg18/LICENSE      (PostgreSQL License, the extension)
  /opt/pg-restrictive-usernames/pg18/LICENSE.restrictive-usernames   (MIT, linked into the .so)
  ```
  ```sh
  buildah copy --from localhost/pg-restrictive-usernames:pg18 "$ctr" \
      /opt/pg-restrictive-usernames/pg18/usr/lib/postgresql/18/lib/ /usr/lib/postgresql/18/lib/
  buildah copy --from localhost/pg-restrictive-usernames:pg18 "$ctr" \
      /opt/pg-restrictive-usernames/pg18/usr/share/postgresql/18/extension/ /usr/share/postgresql/18/extension/
  ```
  The image has no shell, so it is copied from, never run. To list it:
  `podman create --name pru localhost/pg-restrictive-usernames:pg18 true &&
  podman export pru | tar -t; podman rm pru`.
* **`:pg18-cnpg`** (PostgreSQL 18 and later), a
  [CloudNativePG image volume extension](https://cloudnative-pg.io/docs/devel/imagevolume_extensions/):
  `/lib`, `/share/extension` and `/licenses` at the root, non-root user, the
  `io.cloudnativepg.image.*` labels. It needs CloudNativePG with ImageVolume
  support (Kubernetes 1.33 or later) and a cluster on PostgreSQL 18, Debian
  trixie and the same architecture. CloudNativePG's defaults for
  `extension_control_path` and `dynamic_library_path` fit:
  ```yaml
  spec:
    postgresql:
      extensions:
        - name: pg-restrictive-usernames
          image:
            reference: ghcr.io/evancarroll/pg-restrictive-usernames:pg18-cnpg
  ```
  Then `CREATE EXTENSION pg_restrictive_usernames;`, or list it in a
  `Database` resource's `extensions`.
* **`:pg18-postgres`**, `docker.io/library/postgres:18-trixie` with the
  extension installed, to try it:
  ```sh
  podman run -d --name pru -e POSTGRES_PASSWORD=x ghcr.io/evancarroll/pg-restrictive-usernames:pg18-postgres
  podman exec -it pru psql -U postgres -c 'CREATE EXTENSION pg_restrictive_usernames' \
      -c 'SELECT restrictive_usernames.version()'
  ```

Every image carries `org.opencontainers.image.{title,description,source,url,version,authors,revision}`
and `pg-restrictive-usernames.{pg-major,policy-version,unicode-version}`.

### Published images

`ghcr.io/evancarroll/pg-restrictive-usernames`, for `linux/amd64` and
`linux/arm64`, PostgreSQL 14 to 18 (`-cnpg` for 18 only):

| Tag | Moves |
|---|---|
| `pgNN`, `pgNN-cnpg`, `pgNN-postgres` | to each release |
| `<version>-pgNN`, `<version>-pgNN-cnpg`, `<version>-pgNN-postgres` | never |
| `edge-pgNN`, ... | to each push to `main` |

Release images are signed with cosign (keyless, GitHub OIDC) and carry an
SPDX SBOM attestation:

```sh
cosign verify ghcr.io/evancarroll/pg-restrictive-usernames:0.5.0-pg18 \
    --certificate-identity-regexp '^https://github.com/EvanCarroll/restrictive-usernames/' \
    --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

### PGXN

The extension is on [PGXN](https://pgxn.org/dist/pg_restrictive_usernames/)
as a source distribution. It is written in Rust with pgrx, so `pgxn
install` cannot build it. Build it with Rust 1.96 or later, the server
headers of PostgreSQL 14 to 18 (`postgresql-server-dev-NN`, `pg_config` on
`PATH`) and clang:

```sh
unzip pg_restrictive_usernames-0.5.0.zip && cd pg_restrictive_usernames-0.5.0
cargo install --locked cargo-pgrx@0.19.3     # must equal the resolved pgrx version
cargo pgrx init --pg18 "$(which pg_config)"
cd crates/pg_restrictive_usernames
cargo pgrx install --release --pg-config "$(which pg_config)" --no-default-features --features pg18
```

or, with buildah, `PG_MAJOR=18 container/build-extension.sh` from the
unpacked tree. Maintainers: `container/pgxn-bundle.sh v0.5.0` writes
`out/pg_restrictive_usernames-0.5.0.zip` from the tag, after checking that
`META.json` and the workspace agree on the version.

### CI and releases

`.github/workflows/ci.yaml` runs on every push and pull request:
rustfmt, clippy and rustdoc with warnings as errors; the library tests with
and without the policy; the wasm build; the fuzz properties over the
corpus; the minimum Rust version (1.85); `cargo publish --dry-run`; the npm
packages (`build-npm.sh --test --pack`); a one-minute fuzz run per target;
and `build-extension.sh --test` for PostgreSQL 14 to 18 on amd64 and
arm64. On `main` it pushes the `edge-*` images.

It also calls `.github/workflows/opaque-string.yaml` for the password
crate:
- lint, tests (including the exhaustive and differential ones), MSRV and
  `publish --dry-run`;
- `tablegen/regen.sh --check`, which regenerates `src/tables.rs` from the
  checksummed UCD and diffs it;
- the npm package, with its 16 KiB gzip budget and the engine-versus-tables
  NFC comparison;
- a one-minute fuzz run per target.

`.github/workflows/release-opaque-string.yaml` releases that crate from
`opaque-string-v<version>` tags, independently of `v<version>`.

`.github/workflows/release.yaml` runs on a `v<version>` tag that matches
the workspace version. It publishes the crate to crates.io (trusted
publishing), both npm packages (with provenance), the images (signed, with
SBOMs) and the PGXN zip, and creates a GitHub release with the zip and the
release's CHANGELOG entry. Secrets it needs: `PGXN_USERNAME` and
`PGXN_PASSWORD`. crates.io and npm trusted publishing must be configured
for this repository and workflow on each registry first.

## License

* The PostgreSQL extension, `crates/pg_restrictive_usernames/` (Rust
  source, control file, SQL upgrade scripts): the
  [PostgreSQL License](crates/pg_restrictive_usernames/LICENSE).
* Everything else, including the `restrictive-usernames` library, the build
  script, CI and documentation: the [MIT License](LICENSE).

The extension's shared library statically links `restrictive-usernames`, so
a binary distribution of the extension carries both notices. Every image
built by `container/build-extension.sh` includes both: under
`/opt/pg-restrictive-usernames/pgNN/`, `/licenses/pg_restrictive_usernames/`
(CloudNativePG) or `/usr/share/doc/pg_restrictive_usernames/` (the
PostgreSQL image).
