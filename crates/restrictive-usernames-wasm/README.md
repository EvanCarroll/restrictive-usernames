# restrictive-usernames (npm)

A Unicode policy for usernames, nicknames and labels, compiled to
WebAssembly from the Rust library
[`restrictive-usernames`](https://github.com/EvanCarroll/restrictive-usernames),
which also builds the PostgreSQL extension `pg_restrictive_usernames`. The
browser, the server and the database run the same rules.

Two packages, one API:

| Package | Contents | For |
|---|---|---|
| `restrictive-usernames` | `checkStructural` and the vocabulary; no Unicode tables | browsers: instant feedback while a name is typed |
| `restrictive-usernames-full` | adds `check`, `isValid`, `normalize`, the confusable keys | servers, or clients that must agree with the database |

`checkStructural` is always a subset of `check`: show it on every keystroke
and let the server (or the database) report the rest.

## Loading

ES module, built with `wasm-bindgen --target web`. Initialise once before
calling anything.

```js
// Browser or bundler: fetches index_bg.wasm next to index.js.
import init, { checkStructural } from "restrictive-usernames";
await init();
checkStructural("a.b", "username"); // ["disallowed_char"]
```

```js
// Node: pass the bytes.
import { readFileSync } from "node:fs";
import { initSync, normalize, keyCi, keyCs } from "restrictive-usernames-full";
initSync({
  module: readFileSync(new URL(import.meta.resolve("restrictive-usernames-full/index_bg.wasm"))),
});
normalize("Ivan　Smith", "nickname"); // { ok: true, value: "Ivan Smith" }
keyCs("pаypal") === keyCs("paypal"); // true (Cyrillic а)
```

## API

Kinds are `"username" | "nickname" | "label"`; any other string throws.
Rules are their names (`"length"`, `"mixed_script"`, ...), every failed rule
in reporting order; an empty array means valid. `minLen` / `maxLen` override
the kind's default length bounds (code points, inclusive).

Both packages:

* `checkStructural(input, kind, minLen?, maxLen?): RuleName[]`
* `defaultPolicy(kind): { kind, minLen, maxLen, maxMarkRun }`
* `ruleNames(): RuleName[]`: every rule, in reporting order.
* `policyVersion(): string`: equals the database's
  `restrictive_usernames.version()` when both run the same release.

`restrictive-usernames-full` only:

* `check(input, kind, minLen?, maxLen?): RuleName[]`: the storage form.
* `isValid(input, kind, minLen?, maxLen?): boolean`
* `normalize(input, kind, minLen?, maxLen?)`:
  `{ ok: true, value } | { ok: false, rules }`. Ingress: nicknames and
  labels map every space to U+0020, collapse runs and trim; then NFC; then
  `check`. Store `value`.
* `keyCi(input)`, `keyCs(input)`: confusable-skeleton keys; two names
  collide when either key is equal.
* `keyVisualCi(input)`, `keyVisualCs(input)`: the same keys in display order,
  for nicknames.
* `unicodeVersion(): string`

Keys from a different `policyVersion()` than the database's may not match
the stored ones. The rules, the kinds and the upgrade procedure are
documented in the
[repository README](https://github.com/EvanCarroll/restrictive-usernames#readme).

## License

MIT
