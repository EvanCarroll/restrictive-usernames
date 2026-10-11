# Upgrading

How to move a database and its consumers from one release to the next.
[CHANGELOG.md](CHANGELOG.md) says what changed; this file says what to do
about it.

## The REINDEX rule

The keys are only as stable as the rules and tables that compute them. There
are two independent version numbers:

* `POLICY_VERSION` (`restrictive_usernames.version()`): bumped on any rule or
  Unicode table change. Lives in the shared library only.
* the extension version (the crate version, `default_version` in the control
  file, `pg_extension.extversion`): bumped when the **SQL definitions**
  change (signatures, STRICT, defaults) or whenever a release wants a
  recorded upgrade. Each bump ships
  `crates/pg_restrictive_usernames/sql/pg_restrictive_usernames--<old>--<new>.sql`
  (empty except for the `\echo ... \quit` guard if no SQL changes), which
  `cargo pgrx package` copies next to the install script.

The library file name is unversioned, so installing a new image replaces the
code for every extension version at once.

Per deployment, in this order:

1. **Install** the new image's `.so`, `.control` and `.sql` files, then
   restart PostgreSQL so no backend keeps the old library loaded.
2. **Extension object.** If the extension version changed:
   `ALTER EXTENSION pg_restrictive_usernames UPDATE;` (runs the
   `--old--new.sql` script). For a policy-only bump nothing else is needed:
   `SELECT restrictive_usernames.version();` must now show the new policy.
3. **Find rows the new policy rejects** and resolve them (rename, or
   accept by policy exception) before anything relies on the CHECK:
   ```sql
   SELECT id, name FROM users
    WHERE NOT restrictive_usernames.is_valid_nickname(name);
   ```
   or re-validate each domain constraint (`VALIDATE` fails on the first bad
   row without naming it, so run the sweep above to find them):
   ```sql
   ALTER DOMAIN nickname DROP CONSTRAINT nickname_check;
   ALTER DOMAIN nickname ADD CONSTRAINT nickname_check
       CHECK (restrictive_usernames.is_valid_nickname(VALUE)) NOT VALID;
   ALTER DOMAIN nickname VALIDATE CONSTRAINT nickname_check;
   ```
4. **Find key collisions** the new tables create, for every key index (the
   `REINDEX` of a unique index fails on them):
   ```sql
   SELECT restrictive_usernames.key_ci(name), count(*), array_agg(id)
     FROM users GROUP BY 1 HAVING count(*) > 1;
   SELECT restrictive_usernames.key_cs(name), count(*), array_agg(id)
     FROM users GROUP BY 1 HAVING count(*) > 1;
   ```
   (likewise `key_visual_ci` / `key_visual_cs` for a nickname column).
   (add the index's other columns, e.g. a realm id, to `GROUP BY` for a
   composite index). Resolve them, then
5. **`REINDEX INDEX`** every index built on `key_ci`, `key_cs`,
   `key_visual_ci` or `key_visual_cs`.
6. Only then start consumers compiled against the new crate.

Skipping step 5 leaves indexes whose stored keys disagree with the function,
which silently breaks uniqueness and lookups.

## 0.4.0 → 0.5.0 (policy 4 → 5)

* SQL: none (`pg_restrictive_usernames--0.4.0--0.5.0.sql` is empty except
  for its guard). The control file now says `trusted = true`, which only
  changes who may run `CREATE EXTENSION` and `ALTER EXTENSION ... UPDATE`:
  a non-superuser with `CREATE` on the database now can.
* Policy: `key_visual_ci` and `key_visual_cs` apply UAX #9 rule L3, so in
  a right-to-left run a combining mark stays after its base. Their output
  changes for every name with a right-to-left code point and a mark
  (Hebrew points, Arabic harakat and hamza): step 4 must run for every
  `key_visual_*` index, and step 5 is required. `check`, `key_ci` and
  `key_cs` are unchanged, so step 3 finds nothing new and the indexes on
  `key_ci` / `key_cs` need no `REINDEX` (run it anyway if in doubt).
* Rust: `Policy` is `#[non_exhaustive]`. Build one from `Policy::for_kind`
  or a kind's constructor and the `with_min_len`, `with_max_len` and
  `with_max_mark_run` builders (or assign the fields) instead of a struct
  literal. The Unicode table crates are pinned to exact versions, and
  linking other tables is a compile error.

## 0.3.0 → 0.4.0 (policy 3 → 4)

* SQL: adds `key_visual_ci(text)` and `key_visual_cs(text)`
  (`pg_restrictive_usernames--0.3.0--0.4.0.sql`). Nothing is dropped, so
  no domain or index has to be touched for step 2.
* Policy: `check`, `key_ci` and `key_cs` are unchanged for every name, so
  steps 3 and 4 find nothing new and the `REINDEX` of step 5 is a no-op.
  The bump records the UAX #9 tables the new keys read (`unicode_version()`
  gains `bidi 16.0.0`).
* Moving a nickname column from `key_ci` / `key_cs` to the display-order
  keys, after the update:
  1. Find names that collide on the new keys (only names with a
     right-to-left code point can: for the rest the keys are unchanged):
     ```sql
     SELECT restrictive_usernames.key_visual_ci(name), array_agg(id)
       FROM users GROUP BY 1 HAVING count(*) > 1;
     SELECT restrictive_usernames.key_visual_cs(name), array_agg(id)
       FROM users GROUP BY 1 HAVING count(*) > 1;
     ```
     and resolve them.
  2. `CREATE UNIQUE INDEX CONCURRENTLY` the two `key_visual_*` indexes.
  3. Switch every lookup on the column to the `key_visual_*` expressions.
  4. Drop the two `key_ci` / `key_cs` indexes on the column.
  5. Hold every surface that shows the name to the rendering contract.

## 0.2.0 → 0.3.0 (policy 2 → 3)

* SQL: reserved prefixes are gone. `username_rules`, `nickname_rules`,
  `is_valid_username`, `is_valid_nickname` lose their `reserved_prefixes
  text[]` argument and are `STRICT` again; every policy and key function
  gets `COST 50` (`pg_restrictive_usernames--0.2.0--0.3.0.sql`).
* The script drops the `(text, text[])` functions, so any domain CHECK that
  calls them, including one written with the old default
  (`is_valid_nickname(VALUE)` resolved to the two-argument function), must
  be dropped first and re-added against the one-argument function after:
  ```sql
  BEGIN;
  ALTER DOMAIN nickname DROP CONSTRAINT nickname_check;
  ALTER EXTENSION pg_restrictive_usernames UPDATE TO '0.3.0';
  ALTER DOMAIN nickname ADD CONSTRAINT nickname_check
      CHECK (restrictive_usernames.is_valid_nickname(VALUE));
  COMMIT;
  ```
  (likewise for every domain on `is_valid_username`, and any other object
  that depends on the four old functions).
* Policy: `check` and the keys are unchanged for every name under the
  default policies, so steps 3 and 4 find nothing new and the `REINDEX` of
  step 5 is a no-op (run it anyway). The bump is for the structural tier,
  which now refuses ASCII outside the kind's profile, and so that consumers
  checking `version()` refuse an un-upgraded database. A name that relied
  on a reserved prefix being refused (e.g. a nickname `id:x`) is now valid.

## 0.1.0 → 0.2.0 (policy 1 → 2)

* SQL: `username_rules`, `nickname_rules`, `is_valid_username`,
  `is_valid_nickname` become `CALLED ON NULL INPUT`
  (`pg_restrictive_usernames--0.1.0--0.2.0.sql`), so step 2 is required.
* Policy: marks counted by General Category M* (vowel signs with combining
  class 0 now count), the same mark twice in a row and a word starting with
  a mark are refused, nicknames need a letter or digit. Keys are unchanged
  (no table moved), so step 4 finds nothing new, but step 3 can: e.g.
  nicknames `.` or `_`, or stacked Thai or Devanagari vowel signs.
