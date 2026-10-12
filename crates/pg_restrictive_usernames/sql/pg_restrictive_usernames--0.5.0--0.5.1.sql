-- pg_restrictive_usernames 0.5.0 -> 0.5.1
--
-- Policy 5, unchanged: unicode-ident 1.0.27 only re-lays out its tables, so
-- check() and every key are the same for every name. No SQL change.
\echo Use "ALTER EXTENSION pg_restrictive_usernames UPDATE TO '0.5.1'" to load this file. \quit
