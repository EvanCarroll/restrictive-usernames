# Security policy

This library decides which names may exist and which names count as the
same. A way around it is a security bug for every application that relies
on it, even when nothing crashes. Report privately any of these:

* two names that a reader would take for the same (in the rendering the
  README's "Rendering contract" requires) but whose keys all differ, when
  both pass `check`;
* a name that passes `check` but breaks a documented rule (invisible or
  bidi control characters, mixed scripts in one word, Zalgo stacks);
* the Rust library and the PostgreSQL extension disagreeing on any input;
* a panic, hang or unbounded memory use on any input (each one is a
  PostgreSQL `ERROR` or a stalled backend in the extension).

For the password crate `opaque-string`, report privately:

* a password `enforce` accepts for which `prepare` gives different bytes
  on two platforms or backends (native tables, a JavaScript engine that
  passes `engineSupported()`), or in a later release;
* `prepare` not being idempotent, or `enforce` accepting a code point the
  profile refuses;
* an error message, `Debug` output or log line that contains the
  password;
* a panic, hang or unbounded memory use on any input.

Behaviour already listed in [CONFORMANCE.md](CONFORMANCE.md) or under
"Known limitations" in the README is a known gap, not a vulnerability;
issues and pull requests about it are welcome in public.

## Reporting

Email **me@evancarroll.com** with the input (as code points, e.g.
`U+0628 U+0626 U+0628`, since many of these strings do not survive copy
and paste), the policy kind, the version (`restrictive_usernames.version()`
or `POLICY_VERSION`) and what you expected. Please do not open a public
issue first.

You will get an answer within a week. A fix that changes what `check`
accepts or what the keys return is a `POLICY_VERSION` bump; the advisory
will say which indexes to rebuild (UPGRADING.md).

## Supported versions

Only the latest release of each crate (`restrictive-usernames`,
`opaque-string`) gets fixes. Each fix ships as a new release with
upgrade notes, never as a patch to an old policy version.
