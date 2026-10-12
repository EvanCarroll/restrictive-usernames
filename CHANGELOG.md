# Changelog

Every release names its extension version and its policy version
(`POLICY_VERSION`, `restrictive_usernames.version()`); they move
independently. A policy bump means the REINDEX procedure in
[UPGRADING.md](UPGRADING.md). Versions before 0.5.0 were not published.

The password crate `opaque-string` is versioned and released separately;
see [crates/opaque-string/CHANGELOG.md](crates/opaque-string/CHANGELOG.md).

## 0.5.1 (policy 5)

### Changed

* `unicode-ident` 1.0.26 → 1.0.27 (still Unicode 18.0.0). Its tables are
  only re-laid out: `is_xid_start` and `is_xid_continue` agree on every
  code point, so no check result or key changes and no `REINDEX` is needed.

## 0.5.0 (policy 5): first public release

### Fixed

* `key_visual_ci` / `key_visual_cs` apply UAX #9 rule L3: in a
  right-to-left run, combining marks stay after their base. Before, a mark
  landed before its base, so look-alike Arabic nicknames such as `بئب` /
  `بىٔب` did not collide on the display-order keys (CONFORMANCE.md §5.2).
  Changes those keys for right-to-left names with marks: `REINDEX`.

### Changed

* The six Unicode table crates are pinned to exact versions, and a
  compile-time assertion refuses any other tables, so an application and
  the database cannot link different tables under one policy version.
* `Policy` is `#[non_exhaustive]`; new builders `with_min_len` and
  `with_max_mark_run` join `with_max_len`.
* The extension is `trusted`: a non-superuser with `CREATE` on the
  database can install it.
* PostgreSQL 13 (end of life) is no longer a build target; 14 to 18 are.

### Added

* Crate metadata, a crate README and doctests for crates.io and docs.rs.
* `SECURITY.md`.
* Packaging: CloudNativePG extension image, a ready-to-run PostgreSQL
  image, a PGXN distribution, and the npm packages `restrictive-usernames`
  (structural checks) and `restrictive-usernames-full` (the whole policy).

## 0.4.0 (policy 4)

* Added `key_visual_ci` / `key_visual_cs`, the confusable keys of the
  UAX #9 display order, for nicknames that mix directions.

## 0.3.0 (policy 3)

* Removed reserved prefixes from the policy and the SQL API; functions are
  one-argument and `STRICT` again, and declare `COST 50`.
* The structural tier refuses ASCII outside the kind's profile.

## 0.2.0 (policy 2)

* Marks are counted by General Category M*; a repeated mark and a word
  starting with a mark are refused; nicknames need a letter or digit.
* The reserved-prefix functions became `CALLED ON NULL INPUT`.

## 0.1.0 (policy 1)

* Initial version.
