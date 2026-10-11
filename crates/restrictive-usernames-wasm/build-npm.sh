#!/usr/bin/env bash
# Build the two npm packages from this crate:
#
#   restrictive-usernames       no features: structural check, for browsers
#   restrictive-usernames-full  feature check: the full policy and the keys
#
# into target/npm/<package>/, each an ES module for `wasm-bindgen --target
# web`: no bundler or Node-specific loader is needed, the caller awaits
# `init()` (browsers fetch index_bg.wasm next to index.js) or calls
# `initSync({ module: bytes })` (Node, workers).
#
#   build-npm.sh [--test] [--pack]
#     --test  run tests/*.mjs with `node --test` against both packages
#     --pack  `npm pack --dry-run` each package
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

crate_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$crate_dir/../.." && pwd)
cd "$root"

for tool in cargo wasm-bindgen wasm-opt node npm gzip; do
    command -v "$tool" >/dev/null || { echo "error: $tool not found" >&2; exit 1; }
done

# Versions from the resolved graph: the package version (the workspace's)
# and the wasm-bindgen crate that the CLI has to match exactly, because the
# CLI reads the schema the macro wrote into the wasm.
meta=$(cargo metadata --format-version 1)
read -r version bindgen_crate < <(node -e '
    const m = JSON.parse(require("fs").readFileSync(0, "utf8"));
    const pick = n => m.packages.filter(p => p.name === n).map(p => p.version);
    const own = pick("restrictive-usernames-wasm");
    const wb = [...new Set(pick("wasm-bindgen"))];
    if (own.length !== 1 || wb.length !== 1) {
        console.error("error: expected one restrictive-usernames-wasm and one wasm-bindgen, got", own, wb);
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
wasm="$target_dir/wasm32-unknown-unknown/wasm-release/restrictive_usernames_wasm.wasm"
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

# build <npm name> <description> <cargo feature args...>
build() {
    local name=$1 description=$2
    shift 2
    local pkg="$out/$name"

    cargo build -p restrictive-usernames-wasm --target wasm32-unknown-unknown \
        --profile wasm-release "$@"

    rm -rf "$pkg"
    mkdir -p "$pkg"
    wasm-bindgen "$wasm" --out-dir "$pkg" --out-name index --target web \
        --remove-name-section --remove-producers-section
    # Types of the raw wasm exports: internal, not part of the API.
    rm -f "$pkg/index_bg.wasm.d.ts"
    wasm-opt -Oz "${wasm_opt_features[@]}" "$pkg/index_bg.wasm" -o "$pkg/index_bg.wasm"

    cp "$crate_dir/README.md" "$pkg/README.md"
    cp "$root/LICENSE" "$pkg/LICENSE"

    NAME="$name" DESCRIPTION="$description" VERSION="$version" node -e '
        const p = {
            name: process.env.NAME,
            version: process.env.VERSION,
            description: process.env.DESCRIPTION,
            license: "MIT",
            author: "Evan Carroll <me@evancarroll.com>",
            homepage: "https://github.com/EvanCarroll/restrictive-usernames",
            repository: {
                type: "git",
                url: "git+https://github.com/EvanCarroll/restrictive-usernames.git",
                directory: "crates/restrictive-usernames-wasm",
            },
            bugs: "https://github.com/EvanCarroll/restrictive-usernames/issues",
            keywords: [
                "username", "nickname", "validation", "unicode", "uts39",
                "confusables", "homoglyph", "precis", "rfc8265", "rfc8266",
                "wasm",
            ],
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

    printf '%s %s: index_bg.wasm %d bytes, %d gzipped\n' "$name" "$version" \
        "$(wc -c <"$pkg/index_bg.wasm")" "$(gzip -9c "$pkg/index_bg.wasm" | wc -c)"
}

build restrictive-usernames \
    "Unicode username / nickname policy, structural tier for instant browser-side feedback, without the Unicode tables"
build restrictive-usernames-full \
    "Unicode username / nickname policy: UTS #39 identifier profile, Highly Restrictive script mixing, confusable-skeleton keys" \
    --features check

if ((test)); then
    RU_PKG="$out/restrictive-usernames" RU_FULL_PKG="$out/restrictive-usernames-full" \
        node --test "$crate_dir"/tests/*.mjs
fi

if ((pack)); then
    for name in restrictive-usernames restrictive-usernames-full; do
        (cd "$out/$name" && npm pack --dry-run)
    done
fi
