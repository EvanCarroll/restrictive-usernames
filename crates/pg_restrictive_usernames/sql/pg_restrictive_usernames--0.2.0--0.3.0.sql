-- pg_restrictive_usernames 0.2.0 -> 0.3.0
--
-- Policy 2 -> 3 (the rules live in the shared library, which the new image
-- replaces). check() and the keys are unchanged for every name, so the
-- REINDEX of the key indexes is a no-op.
--
-- SQL change: reserved prefixes are removed. The four functions that took a
-- reserved_prefixes text[] are replaced by one-argument STRICT functions,
-- and every policy and key function gets COST 50 (each walks the name
-- through Unicode tables; the C-function default of 1 would have the planner
-- evaluate them before cheaper quals).
--
-- Dependent objects: the DROP FUNCTIONs below fail while anything depends on
-- the two-argument functions, and a domain CHECK written as
-- CHECK (is_valid_nickname(VALUE)) under 0.2.0 does (it resolved to
-- (text, text[]) through the DEFAULT). Drop every such constraint before
-- ALTER EXTENSION ... UPDATE and re-add it afterwards, in one transaction:
--
--   BEGIN;
--   ALTER DOMAIN nickname DROP CONSTRAINT nickname_check;
--   ALTER EXTENSION pg_restrictive_usernames UPDATE TO '0.3.0';
--   ALTER DOMAIN nickname ADD CONSTRAINT nickname_check
--       CHECK (restrictive_usernames.is_valid_nickname(VALUE));
--   COMMIT;
\echo Use "ALTER EXTENSION pg_restrictive_usernames UPDATE TO '0.3.0'" to load this file. \quit

DROP FUNCTION @extschema@.username_rules(text, text[]);
DROP FUNCTION @extschema@.nickname_rules(text, text[]);
DROP FUNCTION @extschema@.is_valid_username(text, text[]);
DROP FUNCTION @extschema@.is_valid_nickname(text, text[]);

-- The definitions below are the ones pgrx generates for 0.3.0
-- (pg_restrictive_usernames--0.3.0.sql), schema-qualified.
CREATE FUNCTION @extschema@."username_rules"(
	"name" TEXT /* & str */
) RETURNS TEXT[] /* Vec < String > */
IMMUTABLE STRICT PARALLEL SAFE COST 50
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'username_rules_wrapper';

CREATE FUNCTION @extschema@."nickname_rules"(
	"name" TEXT /* & str */
) RETURNS TEXT[] /* Vec < String > */
IMMUTABLE STRICT PARALLEL SAFE COST 50
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'nickname_rules_wrapper';

CREATE FUNCTION @extschema@."is_valid_username"(
	"name" TEXT /* & str */
) RETURNS bool /* bool */
IMMUTABLE STRICT PARALLEL SAFE COST 50
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'is_valid_username_wrapper';

CREATE FUNCTION @extschema@."is_valid_nickname"(
	"name" TEXT /* & str */
) RETURNS bool /* bool */
IMMUTABLE STRICT PARALLEL SAFE COST 50
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'is_valid_nickname_wrapper';

ALTER FUNCTION @extschema@.key_ci(text) COST 50;
ALTER FUNCTION @extschema@.key_cs(text) COST 50;
ALTER FUNCTION @extschema@.is_valid_label(text) COST 50;
ALTER FUNCTION @extschema@.label_rules(text) COST 50;
