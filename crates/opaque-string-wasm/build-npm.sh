#!/usr/bin/env bash
# Build the npm package opaque-string from this crate into
# target/npm/opaque-string/: an ES module for `wasm-bindgen --target web`;
# the caller awaits `init()` (browsers fetch index_bg.wasm next to index.js)
# or calls `initSync({ module: bytes })` (Node, workers).
#
# NFC comes from the engine's String.prototype.normalize, so the module
# holds only the refusal table. The build fails if index_bg.wasm gzips to
# more than OPAQUE_STRING_MAX_GZIP bytes (default 16384).
#
#   build-npm.sh [--test] [--pack]
#     --test  also build target/npm/opaque-string-tables (compiled-in NFC,
#             never published) and run tests/*.mjs with `node --test`; the
#             tests compare the engine's NFC with it
#     --pack  `npm pack --dry-run` the package
#
# Needs: rustup target wasm32-unknown-unknown, wasm-bindgen (the CLI version
# must equal the wasm-bindgen crate this crate resolves), wasm-opt (binaryen),
# node, npm, gzip.
set -euo pipefail

test=0
pack=0
for arg in "$@"; do
    case "$arg" in
        --test) test=1 ;;
        --pack) pack=1 ;;
        *) echo "usage: $0 [--test] [--pack]" >&2; exit 2 ;;
    esac
done

max_gzip=${OPAQUE_STRING_MAX_GZIP:-16384}
crate_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$crate_dir/../.." && pwd)
cd "$root"

for tool in cargo wasm-bindgen wasm-opt node npm gzip; do
    command -v "$tool" >/dev/null || { echo "error: $tool not found" >&2; exit 1; }
done

# Versions from the resolved graph: this package's (which must equal the
# opaque-string crate's, as the package is that crate's bindings) and the
# wasm-bindgen crate that the CLI has to match exactly.
meta=$(cargo metadata --format-version 1)
read -r version bindgen_crate < <(node -e '
    const m = JSON.parse(require("fs").readFileSync(0, "utf8"));
    const pick = n => m.packages.filter(p => p.name === n).map(p => p.version);
    const own = pick("opaque-string-wasm");
    const lib = pick("opaque-string");
    const wb = [...new Set(pick("wasm-bindgen"))];
    if (own.length !== 1 || lib.length !== 1 || wb.length !== 1) {
        console.error("error: expected one opaque-string-wasm, opaque-string and wasm-bindgen, got", own, lib, wb);
        process.exit(1);
    }
    if (own[0] !== lib[0]) {
        console.error(`error: opaque-string-wasm ${own[0]} != opaque-string ${lib[0]}`);
        process.exit(1);
    }
    console.log(own[0], wb[0]);
' <<<"$meta")
bindgen_cli=$(wasm-bindgen --version | awk '{print $2}')
if [[ "$bindgen_cli" != "$bindgen_crate" ]]; then
    echo "error: wasm-bindgen CLI $bindgen_cli != wasm-bindgen crate $bindgen_crate" >&2
    echo "       cargo install wasm-bindgen-cli --version $bindgen_crate --locked" >&2
    exit 1
fi

target_dir=$(node -e 'console.log(JSON.parse(require("fs").readFileSync(0, "utf8")).target_directory)' <<<"$meta")
wasm="$target_dir/wasm32-unknown-unknown/wasm-release/opaque_string_wasm.wasm"
out="$target_dir/npm"

# The features rustc enables by default for wasm32-unknown-unknown; wasm-opt
# must accept them in its input and may use them in its output.
wasm_opt_features=(
    --enable-bulk-memory
    --enable-mutable-globals
    --enable-nontrapping-float-to-int
    --enable-sign-ext
    --enable-reference-types
    --enable-multivalue
)

# build <directory name> <cargo feature args...>
build() {
    local name=$1
    shift
    local pkg="$out/$name"

    cargo build -p opaque-string-wasm --target wasm32-unknown-unknown \
        --profile wasm-release "$@"

    rm -rf "$pkg"
    mkdir -p "$pkg"
    wasm-bindgen "$wasm" --out-dir "$pkg" --out-name index --target web \
        --remove-name-section --remove-producers-section
    # Types of the raw wasm exports: internal, not part of the API.
    rm -f "$pkg/index_bg.wasm.d.ts"
    wasm-opt -Oz "${wasm_opt_features[@]}" "$pkg/index_bg.wasm" -o "$pkg/index_bg.wasm"
}

build opaque-string
pkg="$out/opaque-string"
cp "$crate_dir/README.md" "$pkg/README.md"
cp "$root/LICENSE" "$pkg/LICENSE"
VERSION="$version" node -e '
    const p = {
        name: "opaque-string",
        version: process.env.VERSION,
        description: "RFC 8265 OpaqueString, the PRECIS password profile: the same password always becomes the same bytes before OPAQUE or a password hash. NFC from the engine; no Unicode tables shipped.",
        license: "MIT",
        author: "Evan Carroll <me@evancarroll.com>",
        homepage: "https://github.com/EvanCarroll/restrictive-usernames/tree/main/crates/opaque-string",
        repository: {
            type: "git",
            url: "git+https://github.com/EvanCarroll/restrictive-usernames.git",
            directory: "crates/opaque-string-wasm",
        },
        bugs: "https://github.com/EvanCarroll/restrictive-usernames/issues",
        keywords: ["password", "precis", "rfc8265", "opaquestring", "unicode", "nfc", "opaque", "pake", "wasm"],
        type: "module",
        main: "./index.js",
        types: "./index.d.ts",
        exports: {
            ".": { types: "./index.d.ts", default: "./index.js" },
            "./index_bg.wasm": "./index_bg.wasm",
            "./package.json": "./package.json",
        },
        files: ["index.js", "index.d.ts", "index_bg.wasm"],
        sideEffects: false,
    };
    require("fs").writeFileSync(process.argv[1], JSON.stringify(p, null, 2) + "\n");
' "$pkg/package.json"

size=$(wc -c <"$pkg/index_bg.wasm")
gz=$(gzip -9c "$pkg/index_bg.wasm" | wc -c)
printf 'opaque-string %s: index_bg.wasm %d bytes, %d gzipped (budget %d)\n' "$version" "$size" "$gz" "$max_gzip"
if ((gz > max_gzip)); then
    echo "error: index_bg.wasm is $gz bytes gzipped, over the $max_gzip byte budget" >&2
    exit 1
fi

if ((test)); then
    build opaque-string-tables --features nfc-tables
    OS_PKG="$out/opaque-string" OS_TABLES_PKG="$out/opaque-string-tables" \
        OS_CORPUS="$root/crates/opaque-string/tests/corpus.json" \
        node --test "$crate_dir"/tests/*.mjs
fi

if ((pack)); then
    (cd "$pkg" && npm pack --dry-run)
fi
