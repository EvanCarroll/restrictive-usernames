# Conformance to the referenced specifications

The README cites PRECIS (RFC 8264, 8265, 8266), UTS #39, UAX #9, UAX #15,
UAX #31 and Unicode case folding. This project does not claim conformance
to any PRECIS profile. It borrows their shapes and adds a different
repertoire and extra rules. This document lists every place where the
behaviour differs from a spec, says why, and says which way it differs:

* **Stricter**: we refuse, or treat as equal, something the spec accepts
  or treats as distinct.
* **More liberal**: we accept, or treat as distinct, something the spec
  refuses or treats as equal.
* **Different**: neither a subset nor a superset.
* **Unintended**: a defect, not a design choice. To be fixed. None is
  open; §5.2 records the last one, fixed in policy 5.

Spec texts used: RFC 8264, RFC 8265, RFC 8266, RFC 5892, RFC 5893, and UTS #39
version 18.0.0 (2026-08-27). Every example below was run against policy 5
of this crate. RFC behaviour was cross-checked with the `precis-profiles`
0.2.0 crate, whose tables are generated from Unicode 6.3.0, so the examples
use only code points that are assigned there.

## Summary

| Spec item | Here | Direction |
|---|---|---|
| PRECIS repertoire (derived property) | UTS #39 Identifier_Type instead | Different (mostly stricter) |
| PRECIS IdentifierClass ASCII7 grandfathering (`@ ! # ...`) | refused | Stricter |
| PRECIS OtherLetterDigits (Lt, Nl, No) in IdentifierClass | 38 code points admitted | More liberal |
| PRECIS CONTEXTJ (ZWJ, ZWNJ) | always refused | Stricter |
| PRECIS CONTEXTO (U+00B7, U+0375, U+05F3, U+05F4, U+30FB, Arabic-Indic digit sets) | no context rule | More liberal |
| RFC 8265 space-separated userparts | refused | Stricter |
| RFC 8265 width mapping | refused, not mapped | Stricter |
| RFC 8265 case mapping | case preserved; case-insensitive keys | Stricter (more names collide) |
| RFC 8265 Bidi Rule (RFC 5893) | not applied | More liberal |
| RFC 8266 FreeformClass base | identifier profile plus Inclusion punctuation | Stricter |
| RFC 8266 NFKC | NFC storage; compatibility characters refused | Stricter |
| RFC 8266 space mapping | also maps U+2028 and U+2029 | More liberal (ingress only) |
| RFC 8266 "no directionality rule" | display-order keys | Stricter |
| Label vs FreeformClass | Old Hangul Jamo and Cf format characters admitted, Deprecated refused | Different |
| UTS #39 §3.1 General Security Profile | plus Limited_Use (documented tailoring) | More liberal |
| UTS #39 §4 internalSkeleton | Default_Ignorable not removed | Different (no effect on valid names) |
| UTS #39 §4 `skeleton` = bidiSkeleton(LTR) | `key_ci` / `key_cs` use stored order | Different |
| UTS #39 §4 bidiSkeleton(FS) | `key_visual_*` without UBA L4 | Different (no effect on valid names) |
| UTS #39 §5.2 Highly Restrictive | applied per word, not per string | More liberal |
| UTS #39 §5.3 Mixed-Number detection | not implemented; Nl and No admitted | More liberal |
| UTS #39 §5.4 mark sequences | applied to all M*, not only Mn/Me | Stricter |
| UTS #39 §5.4 overlay marks, identical sequences | not implemented | More liberal |
| Leading combining mark | refused per word | Stricter (an addition) |
| UAX #31 identifier start | General Category L* only | Stricter |
| Case folding | full C+F, not toLowerCase | Stricter |
| RFC 8265 OpaqueString (`opaque-string`) CONTEXTJ | refused | Stricter |
| RFC 8265 OpaqueString CONTEXTO | no context rule | More liberal |
| RFC 8265 OpaqueString preparation | class checked on the prepared form, not the input | More liberal |
| RFC 8265 OpaqueString, code points newer than Unicode 12.0 | 145 NFC-unstable ones refused | Stricter |

## 1. PRECIS framework (RFC 8264)

### 1.1 Repertoire comes from UTS #39, not the PRECIS derived property

PRECIS classifies code points with its own algorithm (RFC 8264 §8):
Exceptions, then ASCII7, JoinControl, OldHangulJamo, ignorables, controls,
HasCompat, LetterDigits, and so on. This project classifies them with the
UTS #39 Identifier_Type property instead (`check.rs`, `in_identifier_profile`).

The two systems disagree in both directions. That is why no Kind is a
PRECIS profile.

* **Stricter.** UTS #39 refuses whole classes that PRECIS accepts as
  LetterDigits: Identifier_Type Obsolete, Technical, Uncommon_Use and
  Exclusion (historic scripts, phonetic and liturgical letters).
* **More liberal.** PRECIS puts General Category Lt, Nl, No and Me in
  OtherLetterDigits, which the IdentifierClass refuses. UTS #39 marks 39 of
  them Recommended or Limited_Use. One, U+3007 IDEOGRAPHIC NUMBER ZERO, is
  PVALID anyway through the RFC 5892 Exceptions list. The other 38 are
  admitted here but refused by RFC 8265:
  * U+1F88–U+1F8F, U+1F98–U+1F9F, U+1FA8–U+1FAF, U+1FBC, U+1FCC, U+1FFC
    (Greek capitals with prosgegrammeni, Lt). Example: username `ᾈβγ`.
  * U+A6E6–U+A6EF (Bamum numerals, Nl). Example: username `ꚠꛦꚡ`.
  * U+19DA NEW TAI LUE THAM DIGIT ONE (No).

  Every one of these is single-script, so the script rule never refuses it
  in a word of its own script.

**Why:** UTS #39 is maintained to track which characters are in modern use
and which are spoofing risks, while the PRECIS derived property is close to
a General Category filter. Taking UTS #39 refuses far more spoofing
material than it admits. The 38 extra code points are a side effect, not a
goal. If RFC 8265 compatibility matters, refusing General Category Lt, Nl
(except U+3007) and No in the Username and Nickname profiles closes the
gap at no cost to any modern script.

### 1.2 ASCII7 grandfathering: stricter

RFC 8264 §9.11 makes every printable ASCII code point (U+0021–U+007E)
valid in the IdentifierClass. RFC 8265 §3.1 explicitly allows
`user@example.com` as a username. Here a username admits only
`[A-Za-z0-9_]` from ASCII, and a nickname adds `' - . :`.

**Why:** `@`, `/`, `#`, `!` and the rest of ASCII punctuation are syntax in
URLs, mentions, markup and shells. Keeping them out of names lets an
application use them as escapes without reserving prefixes (README, first
section).

### 1.3 Contextual rules

* **CONTEXTJ: stricter.** RFC 5892 Appendix A allows ZWJ and ZWNJ after a
  virama or between joining letters, e.g. Persian `می‌رود` (U+200C) or
  Devanagari `क्‍` (U+200D after U+094D). Both are UTS #39
  Default_Ignorable, so they are refused in every Kind. The README lists
  this as a known limitation.
* **CONTEXTO: more liberal.** The Nickname profile admits code points that
  PRECIS allows only in context, and checks no context:

  | Code point | RFC 5892 context | Example accepted here, refused by RFC 8266 |
  |---|---|---|
  | U+00B7 MIDDLE DOT | between two `l` (Catalan) | `a·b` |
  | U+30FB KATAKANA MIDDLE DOT | with Hiragana, Katakana or Han | `a・b` |
  | U+0660–U+0669 vs U+06F0–U+06F9 | not both in one string | `ب١۲` |
  | U+0375 GREEK KERAIA | followed by Greek | (Latin before it is refused as mixed script) |
  | U+05F3, U+05F4 GERESH, GERSHAYIM | preceded by Hebrew | (non-Hebrew before it is refused as mixed script) |

  The Highly Restrictive rule covers the cases where the context rule is
  really a script rule (U+0375, U+05F3, U+05F4). It does not cover
  U+00B7, U+30FB or the Arabic digit sets, whose Script_Extensions include
  Common or are shared.

**Why:** the profile was taken from UTS #39 Inclusion, which has no
contextual layer. This is an omission rather than a decision. Adding the
RFC 5892 Appendix A CONTEXTO rules would make it strictly safer.

### 1.4 Refuse instead of map

The PRECIS profiles map some input to a canonical form: width mapping in
RFC 8265 and NFKC in RFC 8266, so `ｂｏｂ` becomes `bob`, `ﬁsh` becomes `fish`,
`Ⅳ` becomes `IV` and `¹ab` becomes `1ab`. Here every such code point is
UTS #39 Not_NFKC and refused (`disallowed_char`). Only spaces are mapped.

**Direction:** stricter. The input is refused, not accepted under another
spelling.

**Why:** the stored name is the one the user typed and sees, and the client
can say what is wrong instead of silently changing it. The keys still apply
NFKC, so a lookup typed as `ｂｏｂ` finds `bob`.

## 2. Usernames vs RFC 8265

| RFC 8265 rule | UsernameCaseMapped | UsernameCasePreserved | `Kind::Username` |
|---|---|---|---|
| Base class | IdentifierClass | IdentifierClass | UTS #39 profile, letters, digits, marks, `_` (§1.1) |
| Spaces | userparts joined by SP | userparts joined by SP | refused |
| Width mapping | map | map | refuse (§1.4) |
| Case mapping | toLowerCase | none | none (storage); folded in `key_ci` |
| Normalization | NFC | NFC | NFC, checked on storage (`not_nfc`) |
| Directionality | Bidi Rule | Bidi Rule | none |
| Comparison | octet equality after enforcement | octet equality after enforcement | equality of `key_ci` **or** `key_cs` |
| Extra rules | none | none | first code point L*, 3–32 code points, Highly Restrictive, marks |

**Relationship.** The Username profile admits no code point that NFKC
changes (measured over all of Unicode). So for an accepted username, width
mapping is a no-op and the stored form is exactly the
UsernameCasePreserved enforcement output. The two exceptions are names that
fail the Bidi Rule (§2.3) and names containing one of the 38 code points in
§1.1, which RFC 8265 refuses.

### 2.1 Userparts: stricter

RFC 8265 §3.1 defines `username = userpart *(1*SP userpart)`, so
`Firstname Lastname` is a username. Here a username is one word (`whitespace`).

**Why:** a username is an address (URL path, mention). Display names with
spaces are the Nickname kind.

### 2.2 Case: stricter

Storage preserves case, as in UsernameCasePreserved. Uniqueness uses
`key_ci`, which makes `ADMIN` and `admin` collide as UsernameCaseMapped
would. It goes further in two ways:

* It uses full case folding instead of toLowerCase, so `Straße` and
  `STRASSE` collide. Under toLowerCase they do not, because `ß` lowercases
  to itself.
* It applies the UTS #39 skeleton, so `pаypal` (Cyrillic а) collides with
  `paypal`.

**Why:** an application needs one answer to "is this name taken", and
case-only and confusable-only differences are impersonation.

### 2.3 Bidi Rule: more liberal

RFC 8265 applies the RFC 5893 Bidi Rule to any username containing a
right-to-left code point. Usernames accepted here that the Bidi Rule
refuses include:

* `אב_`: an RTL label must end in R, AL, EN or AN; `_` is ON.
* `ب1٢`: an RTL label must not contain both EN and AN.

**Why:** the Bidi Rule was written for domain names, where labels joined
by dots must not reorder into each other. A username here is one word in
one script and is shown isolated (README, "Rendering contract"), so it
cannot reorder into neighbouring text. That argument has not been proven
exhaustively. Adopting the Bidi Rule for usernames costs little, since it
refuses mostly trailing punctuation and mixed digit classes. The other
option is to also index `key_visual_ci` / `key_visual_cs` on username
columns.

### 2.4 OpaqueString: the `opaque-string` crate

RFC 8265 §4.2 also defines OpaqueString, for passwords. It is implemented
by a separate crate, `opaque-string`, versioned on its own; §8 covers it.
[PASSWORDS.md](PASSWORDS.md) explains why it is separate.

## 3. Nicknames vs RFC 8266

| RFC 8266 rule | Nickname profile | `Kind::Nickname` |
|---|---|---|
| Base class | FreeformClass | UTS #39 profile plus its Inclusion punctuation |
| Additional mapping | Zs to U+0020, trim, collapse | same, plus U+2028 and U+2029 |
| Normalization | NFKC | NFC; NFKC-unstable code points refused |
| Case mapping | toLowerCase, comparison only | full case folding, in the keys only |
| Directionality | none | display-order keys (`key_visual_*`) |
| Comparison | octet equality after mapping, lowercasing, NFKC | equality of `key_visual_ci` **or** `key_visual_cs` |
| Extra rules | none | 1–32 code points, a letter or digit, Highly Restrictive per word, marks |

### 3.1 Repertoire: stricter

FreeformClass admits symbols, emoji and all punctuation (RFC 8266's own
example is `♚Stpeter`). The Nickname profile admits only letters, marks,
digits and 17 punctuation and symbol code points from UTS #39 Inclusion
(README, "The three kinds"). `ab😀`, `ab♥`, `ab!` and `user@example.com` are
valid RFC 8266 nicknames and are refused here. The CONTEXTO gap in §1.3 is
the one place where this profile is more liberal.

**Why:** a nickname is displayed next to content as the author's identity.
Symbols and emoji are the main tool for imitating badges, verification
marks and staff names. Free text belongs to the Label kind.

### 3.2 NFKC: stricter, and identical on accepted names

RFC 8266 enforcement outputs NFKC. This project stores NFC. The Nickname
profile admits no NFKC-unstable code point (measured over all of Unicode),
so for every accepted nickname NFC and NFKC are the same string. The two
differ only on input that RFC 8266 maps and this project refuses (§1.4).
For the same reason, RFC 8266's "apply the rules until stable" loop is
unnecessary here: NFC cannot create spaces.

### 3.3 Space mapping: more liberal on ingress

`normalize` maps exactly the non-ASCII Zs code points that RFC 8266 maps,
plus U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR, which are Zl and
Zp and therefore DISALLOWED in FreeformClass. The storage check still
refuses them (`whitespace`).

**Why:** they arrive when a name is pasted from multi-line text, and
mapping them gives the same result as for the other separators.

### 3.4 Directionality: stricter

RFC 8266 §2.1 says no directionality rule is needed because "it is
perfectly acceptable for a given nickname to be presented differently in
different layout systems". This project fixes one layout system (README,
"Rendering contract") and compares nicknames in display order under it, so
`Dan אבי` and `אבי Dan` collide. RFC 8266 treats them as distinct.

### 3.5 Extra rules

A nickname must contain a letter or digit, must be Highly Restrictive in
each word (§5.3), and is held to the mark rules (§5.5). RFC 8266 has none
of these.

## 4. Labels vs FreeformClass

No RFC profile covers room, scene or avatar names. The nearest is
FreeformClass, which the Label kind resembles. Both admit symbols, emoji,
punctuation, Lt, Nl, No, Me and compatibility characters, and both refuse
controls, Default_Ignorable code points, noncharacters, private use and
unassigned code points. The differences:

* **More liberal.** Old Hangul Jamo that do not compose (e.g. `ᄓᅡ`,
  U+1113 U+1161): UTS #39 Obsolete, admitted; PRECIS OldHangulJamo,
  DISALLOWED.
* **More liberal.** General Category Cf characters that are not
  Default_Ignorable, e.g. U+0600–U+0605 ARABIC NUMBER SIGN and similar (32
  code points). They fall through to DISALLOWED in RFC 8264 §8 and are
  admitted here.
* **Stricter.** Deprecated characters (e.g. U+0149) are refused here;
  FreeformClass admits them.
* **Stricter.** The mark rules (§5.5) and NFC storage apply.

**Why:** the Label profile is defined as "everything UTS #39 does not mark
as invisible, broken or deprecated". The two admissions above follow from
that definition rather than from a choice about Hangul or Arabic. Refusing
them would align labels with FreeformClass.

## 5. UTS #39 Unicode Security Mechanisms

### 5.1 §3.1 General Security Profile: documented tailoring

UTS-39-C1-2 allows an implementation to add or remove characters from the
profile if it lists them precisely. This section is that list.

* **Added:** Identifier_Type=Limited_Use, restricted to XID_Continue
  (Cherokee, Canadian Syllabics, Mongolian, Tifinagh, Yi, N'Ko and other
  scripts in UAX #31 Table 7). §3.1 itself names these scripts as the
  common candidates for addition.
* **Removed, both kinds:** Recommended and Limited_Use code points without
  XID_Continue. `unicode-security` reports one Identifier_Type per code
  point, so `Limited_Use Not_XID` code points (e.g. Javanese `꧁꧂`) would
  otherwise come back as plain Limited_Use (README, "Known limitations").
* **Removed, usernames:** every Identifier_Type=Inclusion code point. `_`
  is Recommended, so it stays.
* **Joining controls:** not retained, as in the profile since Unicode 15.0.

**Direction:** more liberal than the unmodified profile (Limited_Use), and
otherwise equal or stricter.

**Why:** the Limited_Use scripts have living user communities who need to
write their own names. UTS #39 says these characters "have not been a
priority for examination for confusables", which is a known risk.

The canonical-equivalence clause of §3.1 (`u` + U+0308 is allowed if `ü`
is) is met. No precomposed character is refused while all of its NFD parts
are admitted, measured over all of Unicode for both kinds.

### 5.2 §4 Confusable detection

* **internalSkeleton does not remove Default_Ignorable.** Current UTS #39
  removes Default_Ignorable code points in step 2; `unicode-security`
  0.1 does not, so `pay<U+200B>pal` and `paypal` have different keys.
  Every Kind refuses Default_Ignorable code points, so stored names are
  unaffected. Only lookups on unvalidated text are.
* **Pre-processing (stricter).** The keys apply the RFC 8266 space mapping
  and NFKC before the skeleton, and full case folding around it. UTS #39
  defines no case-insensitive skeleton. Two keys are used because neither
  order of folding and skeleton catches every pair (README, "Keys").
* **`skeleton` is now bidiSkeleton(LTR) (different).** Since Unicode 15.1,
  UTS #39 defines `skeleton(X) = bidiSkeleton(LTR, X)`, which reorders
  right-to-left text for display before mapping. `key_ci` and `key_cs` map
  the stored order, which is the definition from before 15.1. The two agree
  on every string without an R or AL code point. Usernames, the documented
  use of `key_ci` / `key_cs`, are one script per word, so this matters only
  for those keys on mixed-direction text, where the README already directs
  you to `key_visual_*`.
* **`key_visual_*` is bidiSkeleton(FS).** Two gaps remain (2 and 3):
  1. **UBA L3 is applied (fixed in policy 5).** bidiSkeleton moves
     combining marks back after their base once a run is reversed.
     `unicode-bidi`'s `reorder_line` does not, so through policy 4 a mark
     in a right-to-left run landed before its base, the skeleton mapping
     (which can introduce a mark) ran after that, and these pairs of
     **valid** nicknames, which the stored-order keys catch, did not
     collide:

     | a | b | `key_cs` | `key_visual_cs` (policy 4) | `key_visual_cs` (policy 5) |
     |---|---|---|---|---|
     | `بئب` (U+0628 U+0626 U+0628) | `بىٔب` (U+0628 U+0649 U+0654 U+0628) | equal | differ | equal |
     | `بځب` (U+0628 U+0681 U+0628) | `بحٔب` (U+0628 U+062D U+0654 U+0628) | equal | differ | equal |

     `key.rs` now reverses combining sequences (a base and the M* code
     points after it), not code points, within odd-level runs. A unit test
     holds every Bidi_Class NSM mark after six right-to-left bases to that.
     The change moved `key_visual_*` for right-to-left names with marks,
     hence the POLICY_VERSION bump and the REINDEX (UPGRADING.md).
  2. **UBA L4 (mirroring) is not applied.** No code point in the Nickname
     profile is Bidi_Mirrored, so this affects only keys computed over
     other text (README, "Known limitations").
  3. **Paragraphs.** bidiSkeleton treats X as one paragraph in isolation.
     Here each paragraph is reordered on its own. No valid name contains a
     paragraph separator.
* **UTS-39-C2.** The single-script, mixed-script and whole-script
  confusable functions are not implemented, so UTS-39-C2 is not claimed.
  The keys use skeleton equality only.

### 5.3 §5.1, §5.2 Highly Restrictive: per word, more liberal

§5.2 classifies the whole string. Here each space-separated word is
classified on its own (`check.rs`, `highly_restrictive`), so `Иван Smith`
is accepted. As a string it is Cyrillic plus Latin, which is only
Minimally Restrictive, since Moderately Restrictive excludes Cyrillic and
Greek.

**Why:** a nickname is a sequence of personal names, and people have names
in more than one script. Each word on its own still cannot mix scripts, so
`Аdmin` with a Cyrillic А is refused. A username is one word, so for
usernames per-word and per-string are the same. Labels have no script rule
(Unrestricted).

The identifier-profile clause of each restriction level is evaluated
against the tailored profile of §5.1, as §5.2 allows ("an application may
provide an identifier profile").

### 5.4 §5.3 Mixed-Number detection: not implemented, more liberal

* §5.3 refuses strings with digits from more than one decimal system.
  Accepted here: `অ৪8` (Bengali ৪ beside ASCII 8, the spec's own example
  of a confusable pair) and `ب١۲` (Arabic-Indic and Extended Arabic-Indic).
  The script rule catches the cases where the digits carry different
  scripts (`Bob٢`), not those where they share one or are Common.
* §5.3 says only Nd should appear in identifiers. The 11 Nl code points and
  1 No code point listed in §1.1 are admitted.

UTS-39-C5 is not claimed. The README lists this as a known limitation.

### 5.5 §5.4 Optional detection

| §5.4 check | Here | Direction |
|---|---|---|
| no repeated nonspacing mark (Mn, Me) | no mark of any kind (Mn, Mc, Me) twice in a row | Stricter |
| at most 4 nonspacing marks in a row | at most `max_mark_run` (default 4) marks of any kind in a row | Stricter |
| base plus overlaying mark that looks like the base (`i` + U+0307) | not implemented | More liberal |
| distinct sequences with identical rendering (Tamil `ஶ்ரீ` / `ஸ்ரீ`) | not implemented | More liberal |
| CLDR exemplar characters | not implemented | More liberal |
| (none) | every word starts with a non-mark (`no_base`) | Stricter |

**Why:** counting only nonspacing marks lets spacing vowel signs with
combining class 0 stack without limit (Devanagari U+093F, Thai U+0E34). The
`no_base` rule has a precedent in IDNA2008, which refuses a label that
begins with a combining mark (RFC 5891 §4.2.3.2).

## 6. UAX #9, UAX #15, UAX #31, case folding

* **UAX #9.** Paragraph direction comes from the first strong code point
  (P2, P3), as `<bdi>`, `dir="auto"` and FSI…PDI render it. L3 is
  applied, L4 is not (§5.2).
* **UAX #15.** Storage is NFC and is checked, not repaired (`not_nfc`).
  `normalize` produces it. The keys use NFKC.
* **UAX #31.** A username's first code point must be General Category L*
  (Alphabetic ∧ XID_Start ∧ ¬N*). Default identifiers allow any XID_Start,
  which also covers Nl and Other_ID_Start. **Stricter.** All other
  username code points must be XID_Continue.
* **Case folding.** Keys use full default case folding (CaseFolding.txt
  statuses C and F), not toLowerCase, which RFC 8264 §5.2.3 recommends.
  `ß` and `ss` collide (stricter). Both treat `ΣΑΣ` and `σας` alike.
  Turkic mappings (status T) are not used, so `İstanbul` and `istanbul`
  differ, as they do under toLowerCase. The keys do not use NFKC_Casefold,
  which would also remove Default_Ignorable code points (§5.2).

## 7. Unicode versions

PRECIS profiles are defined over the PRECIS derived property values for a
given Unicode version. The comparison crate used here generates its tables
from Unicode 6.3.0. This project's tables come from Unicode 16.0 to 18.0
(README, "Unicode versions"). Code points assigned after 6.3.0 are
classified only by this project. Every other difference in this document
holds at any common version.

## 8. OpaqueString (`opaque-string` crate)

`crates/opaque-string` implements RFC 8265 §4.2: FreeformClass (RFC 8264
§4.3) as the base, non-ASCII spaces (Zs) mapped to U+0020, no width or case
mapping, NFC, no Bidi Rule, and a non-empty result. The class is derived
from UCD 17.0.0 by applying RFC 8264 §8 in its fixed order, with the
RFC 5892 §2.6 Exceptions, including their DISALLOWED entries such as
U+0640 ARABIC TATWEEL (`tablegen/`, checked in CI). A differential test
against `precis-profiles` 0.2, an independent implementation with Unicode
6.3.0 tables, agrees on every code point assigned in 6.3.0 except the
CONTEXTO ones below.

It splits the RFC's single enforcement into two functions:

* `enforce`, for registration and password change, is the RFC's
  enforcement with the deviations below.
* `prepare`, for login, applies only the mapping and NFC and refuses
  nothing. Its output equals `enforce`'s for every password `enforce`
  accepts. Refusing at login would lock out a password that was valid when
  it was set, if a later release refuses more. This is not RFC
  "preparation" (RFC 8264 §6), which checks the class.

Deviations:

* **CONTEXTJ: stricter.** ZWJ and ZWNJ are refused everywhere, as in the
  username profiles (§1.3), so `क्‍` (ZWJ after a virama) and Persian
  `می‌رود` are refused. The RFC accepts them.
* **CONTEXTO: more liberal.** U+00B7, U+0375, U+05F3, U+05F4, U+30FB and
  both Arabic-Indic digit sets are allowed without their RFC 5892 context
  rules (`a·b`, `a・b`, `١۲`). Those rules stop look-alike *displayed*
  strings; a password is never displayed.
* **Class checked on the prepared form: more liberal.** RFC 8265 checks
  FreeformClass on the input before NFC. Here it is checked on the output,
  so `enforce(prepare(x))` and `enforce(x)` agree. The one visible effect:
  a modern Hangul syllable typed as conjoining jamo (`U+1100 U+1161`) is
  accepted as `가`, where the RFC refuses the jamo.
* **NFC baseline: stricter.** Code points newer than Unicode 12.0 that NFC
  treats differently on an engine that does not know them are refused, as
  `unstable_normalization`. That means a non-zero combining class, a
  canonical decomposition, or being part of one. There are 145 for
  Unicode 17.0, mostly combining marks of recent scripts. NFC is stable
  for assigned code points, and an engine treats a code point it does not
  know as a starter without a decomposition, which is correct for every
  other code point. So with these refused, two engines whose Unicode
  versions are both at least 12.0 produce the same bytes for every
  accepted password.
* **Rules applied once.** RFC 8266 asks nickname implementations to repeat
  the rules until stable; RFC 8265 does not, and here the mapping and NFC
  are idempotent (tested exhaustively), so once is enough.

On wasm32 the `js` backend takes NFC from the JavaScript engine
(`String.prototype.normalize`). A canary (U+0EBA, Unicode 12.0, ccc 9)
refuses engines older than the baseline. CI compares Node's NFC with the
compiled-in tables on every accepted code point, alone and in combining
contexts.

