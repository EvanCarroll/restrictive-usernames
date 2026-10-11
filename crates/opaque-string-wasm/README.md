# opaque-string

[RFC 8265 §4.2](https://www.rfc-editor.org/rfc/rfc8265#section-4.2)
OpaqueString, the PRECIS profile for passwords, for JavaScript: the same
password always becomes the same string before it reaches an OPAQUE client
([RFC 9807](https://www.rfc-editor.org/rfc/rfc9807)) or a password hash. It
does no authentication; the name is the RFC's.

NFC comes from the engine's `String.prototype.normalize`, so the module
ships only the refusal table (about 16 KB gzipped). Built from the Rust
crate [`opaque-string`](https://crates.io/crates/opaque-string), which gives
the same results with compiled-in tables on servers and native clients.

```js
import init, { enforce, prepare, engineSupported } from "opaque-string";
await init(); // Node or a worker: initSync({ module: bytes })

// Registration and password change.
const r = enforce("Café 2024");
// { ok: true, value: "Café 2024" }
// or { ok: false, reason: "control", codePoint: 9, index: 4, count: 1 }
// or { ok: false, reason: "empty" } / { ok: false, reason: "lone_surrogate" }

// Login: the same preparation, refusing nothing.
const bytes = new TextEncoder().encode(prepare(password));
```

* Call `enforce` when a password is set and `prepare` when it is used.
  `prepare` refuses nothing, so a password accepted once keeps working.
* `engineSupported()` is `false` on an engine whose normalization predates
  Unicode 12.0 (`nfcBaseline()`); `prepare` and `enforce` then throw rather
  than risk bytes that differ from other devices.
* Strings with an unpaired surrogate are refused, not silently turned into
  U+FFFD.
* `reason` is one of `reasonNames()`: `unassigned`, `control`, `ignorable`,
  `old_hangul_jamo`, `join_control`, `exception`, `disallowed`,
  `unstable_normalization`.

The deviations from RFC 8265 and the reasons are in the crate's README.
MIT license.
