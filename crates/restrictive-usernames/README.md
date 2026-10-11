# restrictive-usernames

A Unicode policy for usernames, nicknames and labels, and confusable keys
for enforcing uniqueness. The same code also ships as a PostgreSQL extension
(`pg_restrictive_usernames`), so the application and the database apply the
same rules and compute the same keys.

* **Identifier profile** (UTS #39 §3.1): letters, digits, marks and a short
  list of punctuation. No emoji, no symbols, no invisibles, no compatibility
  look-alikes (`ﬁ`, fullwidth, `𝓑𝓸𝓫`).
* **Highly Restrictive** script mixing (UTS #39 §5.2), per word: `Аdmin` with
  a Cyrillic `А` is refused; `ローマ字Latin` and `Иван Smith` are accepted.
* **Marks**: no Zalgo stacks, no repeated mark, no mark without a base.
* **Confusable keys** (UTS #39 §4 skeletons) for unique indexes: `paypal`
  and `pаypal` cannot both exist, and neither can `Dan אבי` and `אבי Dan`,
  which display the same.

```rust
use restrictive_usernames::{Policy, Rule, check, key_ci, key_cs, normalize};

let nick = Policy::nickname();
assert_eq!(normalize("Jean\u{3000}Luc", &nick).unwrap(), "Jean Luc");
assert_eq!(check("\u{410}dmin", &nick), [Rule::MixedScript]);

// Two names collide when either key does.
assert_eq!(key_ci("ADMIN"), key_ci("admin"));
assert_eq!(key_cs("p\u{430}ypal"), key_cs("paypal"));
```

| Kind | Allowed | Length (default) |
|---|---|---|
| `username` | letters, marks, digits, `_`; first a letter; one word | 3 to 32 |
| `nickname` | the identifier profile plus `' - . :` and similar; words joined by single spaces | 1 to 32 |
| `label` | anything but invisibles, noncharacters, deprecated code points and controls | 1 to 50 |

## Features

* `check` (default): the full policy (`check`, `is_valid`, `normalize`, and
  the keys `key_ci`, `key_cs`, `key_visual_ci`, `key_visual_cs`).
* Without it: the rule vocabulary and `check_structural`, which needs no
  Unicode tables (serde and thiserror only; about 19 KB of gzipped wasm in
  the npm package `restrictive-usernames`), for instant feedback in a
  browser.
* `corpus`: the acceptance corpus as `pub const` tables.

## Stability

The keys back unique indexes, so they must not change behind your back. The
Unicode table crates are pinned to exact versions, linking other tables is a
compile error, and any change to a rule or a table bumps `POLICY_VERSION`.
After a bump, the stored keys must be rebuilt; see
[UPGRADING.md](https://github.com/EvanCarroll/restrictive-usernames/blob/main/UPGRADING.md).

Minimum Rust: 1.85.

## More

* [Project README](https://github.com/EvanCarroll/restrictive-usernames#readme):
  every rule, the keys, the PostgreSQL extension, the rendering contract for
  right-to-left names.
* [CONFORMANCE.md](https://github.com/EvanCarroll/restrictive-usernames/blob/main/CONFORMANCE.md):
  every difference from PRECIS (RFC 8264, 8265, 8266), UTS #39 and UAX #9.
* [SECURITY.md](https://github.com/EvanCarroll/restrictive-usernames/blob/main/SECURITY.md):
  reporting a bypass.

License: MIT.
