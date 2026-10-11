# Changelog

opaque-string is versioned independently of restrictive-usernames and is
released from `opaque-string-v<version>` tags. Each entry names the Unicode
version of the refusal table and the NFC baseline.

## 0.1.0 (Unicode 17.0.0, NFC baseline 12.0): first release

* `enforce` (registration) and `prepare` (login): RFC 8265 §4.2
  OpaqueString over RFC 8264 FreeformClass, generated from UCD 17.0.0
  (`tablegen/regen.sh`).
* Deviations: CONTEXTJ refused, CONTEXTO allowed, the class checked on the
  prepared form, and 145 NFC-unstable code points newer than Unicode 12.0
  refused.
* Backends: compiled-in NFC (`nfc-tables`, default) or the JavaScript
  engine's (`js`, wasm32), with an engine canary.
* `Prepared` zeroes its buffer on drop; errors never contain the password.
* npm package `opaque-string`.
