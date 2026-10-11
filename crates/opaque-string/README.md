# opaque-string

[RFC 8265 §4.2](https://www.rfc-editor.org/rfc/rfc8265#section-4.2)
OpaqueString, the PRECIS profile for passwords: the same password always
becomes the same bytes, on every device, before it reaches OPAQUE
([RFC 9807](https://www.rfc-editor.org/rfc/rfc9807), e.g. `opaque-ke`) or a
password hash.

This crate does no authentication; it is the step before it. The name is
the RFC's, not a reference to the OPAQUE protocol.

```rust
// Registration and password change: refuse what the profile refuses.
let registered = opaque_string::enforce("Cafe\u{0301} 2024").unwrap();
assert_eq!(registered.as_str(), "Caf\u{E9} 2024"); // e + U+0301 becomes é

// Login: the same preparation, refusing nothing.
let login = opaque_string::prepare("Café\u{00A0}2024"); // a pasted no-break space
assert_eq!(login.as_bytes(), "Café 2024".as_bytes());
```

## What it does

* **Maps** non-ASCII spaces (General Category Zs) to U+0020, so a no-break
  space from a password manager matches a typed space.
* **Applies NFC**, so `é` typed precomposed on one device and as `e` +
  U+0301 on another give the same bytes.
* **Keeps** case, width and everything else (no NFKC, no case folding): the
  password keeps its entropy.
* **Refuses** at registration ([`enforce`]) what RFC 8264's FreeformClass
  refuses: controls, invisible and default-ignorable characters,
  noncharacters, private use, unassigned code points, conjoining old Hangul
  jamo, and the RFC 5892 exceptions such as U+0640 ARABIC TATWEEL.

## Enforce at registration, prepare at login

Call `enforce` when a password is set and `prepare` when it is used.
`prepare` never refuses. For any password `enforce` accepted, `prepare`
returns the same bytes on every platform and in every later release, even
if a later release refuses more at registration. Refusing at login could
lock a user out of a password that was valid when it was set.

With OPAQUE, both run on the client: the server never sees the password,
so it cannot normalize it. Any two of a user's devices must agree to the
byte, which is what this crate is for.

## Deviations from RFC 8265

[CONFORMANCE.md §8](https://github.com/EvanCarroll/restrictive-usernames/blob/main/CONFORMANCE.md#8-opaquestring-opaque-string-crate)
has the details. In brief:

* ZWJ and ZWNJ are refused everywhere (the RFC allows them in context).
* The RFC 5892 CONTEXTO code points (U+00B7, U+0375, U+05F3, U+05F4,
  U+30FB, the Arabic-Indic digits) are allowed anywhere: their context
  rules exist to stop look-alike *displayed* strings, and passwords are
  never displayed.
* The class is checked on the prepared password, not the raw input, so a
  modern Hangul syllable typed as conjoining jamo is accepted.
* 145 code points newer than Unicode 12.0 that NFC treats differently on
  an engine that does not know them (mostly combining marks of recent
  scripts) are refused, so every engine produces the same bytes.

## Backends and the browser

* **`nfc-tables`** (default): NFC from compiled-in tables (Unicode 17.0.0).
  For servers and native clients.
* **`js`**, without `nfc-tables`, on wasm32: NFC from the JavaScript
  engine's `String.prototype.normalize` (core ECMAScript, not `Intl`), so
  the wasm module carries no normalization tables. The engine must know
  Unicode 12.0: `enforce` returns `Refused::UnsupportedEngine` otherwise,
  and `engine_supported()` says so in advance.

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
opaque-string = { version = "0.1", default-features = false, features = ["js"] }
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
opaque-string = "0.1"
```

The npm package `opaque-string` is this crate with the `js` backend, about
16 KB gzipped.

Version skew between engines shows up only at registration, as a refusal,
never as different bytes: NFC is stable for assigned code points, and the
code points whose normalization an older engine would get wrong are
exactly the ones refused. CI checks Node's NFC against the compiled-in
tables on every accepted code point.

## Handling the password

* `Prepared` zeroes its buffer on drop, has no `Display`, and its `Debug`
  prints `Prepared(..)`. The caller's input, JavaScript strings and the
  normalizer's internal buffers are not covered.
* Errors never contain the password. They name the first refused code
  point as `U+XXXX`, which cannot appear in an accepted password.
* Cap the input length before calling either function; the crate does not.
* Strength and length rules are yours. With OPAQUE they can only run on
  the client.

## Versioning

Released independently of `restrictive-usernames`, from `opaque-string-v*`
tags. A change of Unicode version is a minor release: it can only change
what `enforce` refuses, never what `prepare` returns for an accepted
password. See [CHANGELOG.md](CHANGELOG.md).

## License

MIT.

[`enforce`]: https://docs.rs/opaque-string/latest/opaque_string/fn.enforce.html
