# Passwords: OPAQUE, OpaqueString, and where normalization belongs

This note records what was measured and decided before writing the
`opaque-string` crate (`crates/opaque-string`, npm `opaque-string`): what an
OPAQUE client costs in the browser, how the RFC 8265 OpaqueString profile
fits in, and why the implementation is a separate crate. The crate's README
and CONFORMANCE.md §8 describe what was built.

The two names are unrelated:

* **OPAQUE** (RFC 9807) is an asymmetric password-authenticated key
  exchange. The server stores a registration record and never sees the
  password, not even during login.
* **OpaqueString** (RFC 8265 §4.2) is the PRECIS profile for passwords. It
  turns typed text into a stable byte string before anything hashes it. It
  is "opaque" because it does not case-map or fold the password the way
  the username profiles do.

## Measurements

These are wasm32 release builds (`opt-level = "z"`, LTO, `panic =
"abort"`), processed with `wasm-bindgen --target web` where the crate uses
it, then `wasm-opt -Oz`. Each probe exports only the functions named, so
nothing is removed as dead code. Every module carries its own allocator
and panic handling, so these are standalone upper bounds; inside an
application that already has those, the added size is smaller.

| Build | After wasm-opt | gzip |
|---|---|---|
| `restrictive-usernames`, structural checks only (the current browser build) | 16.0 KB | 7.4 KB |
| structural + an OpaqueString sketch (space map, NFC, refusals by `Identifier_Type`) | 167.5 KB | 80.2 KB |
| `precis-profiles` 0.2 `OpaqueString::enforce` | 198.8 KB | 87.6 KB |
| OPAQUE client, `opaque-ke` 3.0, ristretto255, 3DH, **no Argon2** (KSF = Identity) | 79.3 KB | 35.6 KB |
| the same OPAQUE client with Argon2 | 94.6 KB | 40.6 KB |
| `restrictive-usernames` full `check` and keys | 409.0 KB | 176.7 KB |

The OPAQUE client probe covers both registration (start, finish) and login
(start, finish). Its size is mostly ristretto255 arithmetic, SHA-512 and
the protocol itself.

Before `wasm-bindgen` processes it, the same module is about 420 KB raw:
`getrandom`'s `js` feature brings in a 180 KB `__wasm_bindgen_unstable`
custom section that the CLI strips. Measure after `wasm-bindgen`, never
from `cargo build` output.

### The OPAQUE client without Argon2

Dropping Argon2 saves about 5 KB gzipped (mostly BLAKE2b), and it is not
worth it. In OPAQUE the key stretching function (KSF) runs on the client,
and it is the only thing that slows an offline dictionary attack once the
server's credential file is stolen (RFC 9807 §10.8). With the Identity KSF,
anyone holding the stolen file and the server's OPRF key can test guesses
at the speed of a few curve operations each. RFC 9807 uses Identity only in
its test vectors (Appendix C). Its recommended configurations use Argon2id
or scrypt.

Keep Argon2, and lower its parameters instead. The RFC's recommended
Argon2id memory, `m = 2^21` KiB (2 GiB), cannot run in a browser tab on a
phone. §10.8 leaves the cost-versus-protection tradeoff to the
application, so pick parameters your slowest supported client can finish
in about a second.

## How OpaqueString fits

RFC 9807 treats the password as "an opaque byte string". It says nothing
about how text becomes bytes. That is OpaqueString's job:

1. Map non-ASCII spaces (General Category Zs) to U+0020, because a
   non-breaking space pasted from a password manager must match a typed
   space.
2. Apply NFC, so `é` typed precomposed on one device and decomposed
   (`e` + U+0301) on another produce the same bytes.
3. Refuse what FreeformClass refuses: controls, unassigned code points,
   noncharacters, Default_Ignorable code points, old Hangul jamo.
4. Do no case mapping, no width mapping and no NFKC, so the password keeps
   its entropy.

Where it must run depends on who turns the password into bytes:

| Design | Who needs OpaqueString | Notes |
|---|---|---|
| Plaintext over TLS, server hashes (Argon2id) | the server only | the browser does nothing; 0 bytes of wasm |
| OPAQUE | **every client**, before `ClientRegistration::start` and `ClientLogin::start` | the server never sees the password, so it cannot normalize |

With OPAQUE, a normalization mismatch between two of a user's devices is a
lockout. The server cannot try "raw, then normalized" the way a
server-hashing design can when migrating old hashes. Only the client can
retry, and each retry counts as another online login attempt.

### Enforce at registration, prepare at login

Make the refusals (step 3) at registration and password change only. At
login, apply steps 1 and 2 and send the result, refusing nothing. Two
reasons:

* NFC never changes for an assigned character (the Unicode normalization
  stability policy). Steps 1 and 2 therefore give the same bytes on every
  Unicode version for any password that was accepted at registration,
  because registration refused unassigned code points.
* The refusal sets can change between Unicode versions. UTS #39 says
  outright that Identifier_Status has no stability guarantees. A refusal
  at login after a table update would lock out a password that was valid
  when set.

With this split, Unicode version skew between devices can only surface at
registration, as a refusal ("not allowed") on the older device. It can
never make two devices produce different bytes for the same password.

### In the browser: the engine's NFC, not compiled-in tables

An OPAQUE login page needs the OPAQUE client (about 41 KB gzipped) and
OpaqueString. Compiled with its own NFC tables, OpaqueString adds about
73 KB gzipped, more than the OPAQUE client itself. The browser already
ships those tables, so the crate's `js` backend calls the engine's
`String.prototype.normalize` and the npm package carries only the refusal
table: **15.6 KB gzipped**, as measured by `build-npm.sh`, about half of
which is the allocator a host application already has.

An earlier draft of this note used a hand-written JavaScript snippet
instead. That approach was dropped for three reasons:
- it missed the RFC 5892 DISALLOWED exceptions (U+0640 tatweel and nine
  more);
- its `\p{Cn}` meant "unassigned in *this engine's* Unicode version", so
  two browsers disagreed about what to refuse;
- it had no answer to engines whose NFC is older than the server's.

The crate solves the last point with an NFC baseline (Unicode 12.0):
- registration refuses the 145 newer code points whose normalization an
  older engine would get wrong;
- a canary refuses engines older than the baseline.

CI compares Node's NFC with the compiled-in tables on every accepted code
point. One Rust implementation serves the browser (`js`), servers and
native clients (`nfc-tables`).

`precis-profiles` is not a good fit for either side. It is the largest
option measured, and its tables are Unicode 6.3.0, so it refuses every
code point assigned since 2013, including most emoji. It is still useful
as an independent check: the crate's differential test agrees with it on
every code point assigned in 6.3.0 except the documented CONTEXTO
deviation.

## Why a separate crate

`opaque-string` is a separate crate in this workspace, not a module or
feature of `restrictive-usernames`, and **not part of the PostgreSQL
extension**:

* **The stability contract is different.** `restrictive-usernames` may
  change its output at any POLICY_VERSION bump, and the REINDEX procedure
  repairs indexes after a bump. Nothing repairs a password hash or an
  OPAQUE registration record, because the plaintext is gone. So
  `opaque-string` versions on its own (`opaque-string-v*` tags), and a
  Unicode update may only change what `enforce` refuses, never what
  `prepare` returns for an accepted password.
* **The dependencies are different.** OpaqueString needs NFC and its own
  table, not confusables, scripts, bidi or case folding.
* **Passwords must not reach SQL.** A plaintext password in a query ends up
  in logs and `pg_stat_statements`. The extension does not depend on
  `opaque-string`, so no feature flag can expose it as a SQL function.
* **Little is shared.** Only the Zs space list and the
  `unicode-normalization` pin are common.

## In chattyness

chattyness logs in with OPAQUE and prepares passwords with this crate. The
OPAQUE configuration lives in one crate, `chattyness-opaque`, which the
browser, the server and `chattyness-cli` all compile, so they cannot drift:
- ristretto255 for the OPRF and the 3DH key exchange;
- client key stretching with Argon2id at m = 19 MiB, t = 2, p = 1;
- a fixed context string.

* **Browser.** `chattyness-opaque/client` uses `opaque-string` with the `js`
  backend: the password is prepared with the engine's NFC, stretched and
  proven in wasm, and never sent.
* **Server.** It stores only each account's OPAQUE record. It keeps the
  login state between the two round trips in a one-minute,
  single-use database row, and answers an unknown username with a fake
  response of the same shape.
* **Temporary passwords are gone.** Admins, the CLI and new realm owners
  get one-time setup tokens, redeemed at `/setup?token=` or with
  `chattyness-cli account setup`. Changing a password needs a reauth (an
  OPAQUE login), which earns a single-use change token.
* **Bots have no password.** They authenticate with bearer tokens, accepted
  on the user API and the realm WebSocket only for `auth.bots` accounts.

The server's OPAQUE secret (`OPAQUE_SERVER_SETUP`, from
`chattyness-app opaque-setup`) is the one thing that must never be lost or
rotated casually: every stored record depends on it.
