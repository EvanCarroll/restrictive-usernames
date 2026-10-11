-- pg_restrictive_usernames 0.3.0 -> 0.4.0
--
-- Policy 3 -> 4 (the rules live in the shared library, which the new image
-- replaces). check(), key_ci() and key_cs() are unchanged for every name, so
-- the REINDEX of the existing key indexes is a no-op.
--
-- SQL change: two new functions, the confusable keys of the displayed form
-- (UAX #9 visual order, paragraph direction from the first strong code
-- point, as <bdi>, dir="auto" and U+2068 FSI render a name). Nothing is
-- dropped, so dependent domains and indexes are untouched.
\echo Use "ALTER EXTENSION pg_restrictive_usernames UPDATE TO '0.4.0'" to load this file. \quit

-- The definitions below are the ones pgrx generates for 0.4.0
-- (pg_restrictive_usernames--0.4.0.sql), schema-qualified.
CREATE FUNCTION @extschema@."key_visual_ci"(
	"name" TEXT /* & str */
) RETURNS TEXT /* String */
IMMUTABLE STRICT PARALLEL SAFE COST 50
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'key_visual_ci_wrapper';

CREATE FUNCTION @extschema@."key_visual_cs"(
	"name" TEXT /* & str */
) RETURNS TEXT /* String */
IMMUTABLE STRICT PARALLEL SAFE COST 50
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'key_visual_cs_wrapper';
