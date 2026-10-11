#!/usr/bin/env bash
# Build the PGXN distribution, out/pg_restrictive_usernames-<version>.zip,
# from a git tree-ish (default HEAD) with `git archive`.
#
#   container/pgxn-bundle.sh [TREE-ISH] [--out DIR]
#
#   TREE-ISH   commit, tag or tree to bundle (default HEAD)
#   --out DIR  where to write the zip (default <checkout>/out)
#
# PGXN wants one top-level directory named <name>-<version>/ holding
# META.json. The whole workspace goes in: the extension crate builds the
# library crate from its path dependency, and cargo pgrx needs the root
# Cargo.toml for the workspace version and the panic = "unwind" profiles.
# Only committed content is read (META.json and Cargo.toml included), so
# the zip is what the tag says, whatever the working tree holds.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REF=HEAD
OUT_DIR="${REPO}/out"

usage() { sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
    case "$1" in
        --out)
            [ $# -ge 2 ] || { echo "--out needs a directory" >&2; exit 2; }
            OUT_DIR="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        -*) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
        *) REF="$1"; shift ;;
    esac
done

git() { command git -C "$REPO" "$@"; }
die() { echo "$*" >&2; exit 1; }

git rev-parse --verify --quiet "${REF}^{tree}" >/dev/null || die "not a tree-ish: ${REF}"

META="$(git show "${REF}:META.json")" || die "${REF} has no META.json"
NAME="$(jq -er '.name' <<<"$META")" || die "META.json: no name"
META_VERSION="$(jq -er '.version' <<<"$META")" || die "META.json: no version"

# The workspace version, from [workspace.package] in the root Cargo.toml.
# Read the committed file with sed rather than cargo metadata, which would
# read the working tree.
CARGO_VERSION="$(git show "${REF}:Cargo.toml" | sed -n '/^\[workspace\.package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p}')"
[ -n "$CARGO_VERSION" ] || die "cannot read [workspace.package] version from ${REF}:Cargo.toml"

[ "$META_VERSION" = "$CARGO_VERSION" ] \
    || die "META.json version ${META_VERSION} != workspace version ${CARGO_VERSION}"
# Each provided extension's version is the extension version that
# CREATE EXTENSION installs (default_version = @CARGO_VERSION@).
BAD="$(jq -r --arg v "$CARGO_VERSION" '.provides | to_entries[] | select(.value.version != $v) | "\(.key) \(.value.version)"' <<<"$META")"
[ -z "$BAD" ] || die "META.json provides a version other than ${CARGO_VERSION}: ${BAD}"
[ "$(jq -r '."meta-spec".version' <<<"$META")" = 1.0.0 ] || die "META.json: meta-spec.version must be 1.0.0"

EXT_DIR=crates/pg_restrictive_usernames
FILES="$(git ls-tree -r --name-only "$REF")"
# What a cargo pgrx install from the unpacked zip reads, plus the license
# notices and the documentation META.json points at.
for f in \
    META.json README.md LICENSE Cargo.toml \
    "${EXT_DIR}/Cargo.toml" "${EXT_DIR}/LICENSE" "${EXT_DIR}/${NAME}.control" "${EXT_DIR}/src/lib.rs" \
    crates/restrictive-usernames/Cargo.toml crates/restrictive-usernames/LICENSE \
    crates/restrictive-usernames/src/lib.rs \
    $(jq -r '.provides[] | .file, (.docfile // empty)' <<<"$META"); do
    grep -qxF "$f" <<<"$FILES" || die "${REF} lacks ${f}"
done
# cargo reads every workspace member's manifest before building any one of
# them, so a member missing from the tree breaks the build of the extension.
MEMBERS="$(git show "${REF}:Cargo.toml" | sed -n '/^members = \[/,/\]/p' | grep -o '"[^"]*"' | tr -d '"')"
[ -n "$MEMBERS" ] || die "cannot read workspace members from ${REF}:Cargo.toml"
for m in $MEMBERS; do
    grep -qxF "${m}/Cargo.toml" <<<"$FILES" || die "${REF} lacks workspace member ${m}/Cargo.toml"
done
# Every release after the first ships the upgrade script into it.
if ! grep -qE "^${EXT_DIR}/sql/${NAME}--[^-]+--${CARGO_VERSION//./\\.}\.sql$" <<<"$FILES"; then
    die "${REF} lacks ${EXT_DIR}/sql/${NAME}--<old>--${CARGO_VERSION}.sql"
fi

PREFIX="${NAME}-${CARGO_VERSION}"
ZIP="${OUT_DIR}/${PREFIX}.zip"
mkdir -p "$OUT_DIR"
git archive --format=zip --prefix="${PREFIX}/" -o "$ZIP" "$REF"
# Listed into a variable: grep -q on a pipe would exit early and fail the
# pipeline (pipefail) with unzip's SIGPIPE.
LISTING="$(unzip -Z1 "$ZIP")"
grep -qxF "${PREFIX}/META.json" <<<"$LISTING" || die "${ZIP}: no ${PREFIX}/META.json"
echo "==> ${ZIP} ($(grep -vc '/$' <<<"$LISTING") files, from ${REF})"
