#!/usr/bin/env bash
# Regenerate crates/opaque-string/src/tables.rs from the Unicode Character
# Database, or with --check prove that the committed file is what this
# generator produces.
#
#   regen.sh [--check]
#
# The UCD files are fetched with curl into a fresh temporary directory and
# must match ucd-$UNICODE.sha256 before anything reads them. Moving to a new
# Unicode version means changing UNICODE (it must equal the version of the
# pinned unicode-normalization crate; the generator refuses anything else),
# recording new checksums with --record-checksums, and a minor release of
# opaque-string.
#
#   regen.sh --record-checksums   fetch, print sha256sum lines, change nothing
#
# Needs: cargo, curl, sha256sum, rustfmt, diff.
set -euo pipefail

UNICODE=17.0.0
BASELINE=12.0
FILES=(UnicodeData DerivedAge DerivedCoreProperties HangulSyllableType PropList)

mode=install
case "${1:-}" in
    "") ;;
    --check) mode=check ;;
    --record-checksums) mode=record ;;
    *) echo "usage: $0 [--check | --record-checksums]" >&2; exit 2 ;;
esac

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
crate=$(cd "$here/.." && pwd)
target="$crate/src/tables.rs"

for tool in cargo curl sha256sum rustfmt diff; do
    command -v "$tool" >/dev/null || { echo "error: $tool not found" >&2; exit 1; }
done

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

for f in "${FILES[@]}"; do
    curl -fsSL --proto '=https' --retry 3 \
        -o "$work/$f.txt" "https://www.unicode.org/Public/${UNICODE}/ucd/$f.txt"
done

if [[ $mode == record ]]; then
    (cd "$work" && sha256sum ./*.txt | sed 's| \./| |')
    exit 0
fi

(cd "$work" && sha256sum --quiet -c "$here/ucd-${UNICODE}.sha256")

cargo run --quiet --release --manifest-path "$here/Cargo.toml" -- \
    --ucd "$work" --unicode "$UNICODE" --baseline "$BASELINE" \
    --checksums "$here/ucd-${UNICODE}.sha256" --out "$work/tables.rs"
rustfmt --edition 2024 "$work/tables.rs"

if [[ $mode == check ]]; then
    if diff -u "$target" "$work/tables.rs"; then
        echo "src/tables.rs is up to date (Unicode $UNICODE, baseline $BASELINE)"
    else
        echo "error: src/tables.rs differs from the generator output; run $0" >&2
        exit 1
    fi
else
    cp "$work/tables.rs" "$target"
    echo "wrote $target"
fi
