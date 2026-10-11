-- pg_restrictive_usernames 0.1.0 -> 0.2.0
--
-- Policy 1 -> 2 (the rules live in the shared library, which the new image
-- replaces; nothing to do for them here).
--
-- SQL change: the four functions that take reserved_prefixes are no longer
-- STRICT. Under STRICT an explicit NULL array made them return NULL, and a
-- domain CHECK that evaluates to NULL passes. They now return NULL only for
-- a NULL name and raise an error for a NULL array.
\echo Use "ALTER EXTENSION pg_restrictive_usernames UPDATE TO '0.2.0'" to load this file. \quit

ALTER FUNCTION @extschema@.username_rules(text, text[]) CALLED ON NULL INPUT;
ALTER FUNCTION @extschema@.nickname_rules(text, text[]) CALLED ON NULL INPUT;
ALTER FUNCTION @extschema@.is_valid_username(text, text[]) CALLED ON NULL INPUT;
ALTER FUNCTION @extschema@.is_valid_nickname(text, text[]) CALLED ON NULL INPUT;
