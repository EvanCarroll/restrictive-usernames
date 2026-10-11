#!/usr/bin/env bash
# Push the images build-extension.sh committed, and join the per-architecture
# pushes into multi-arch manifests. CI runs `arch` on every runner, then
# `manifest` once per PostgreSQL major.
#
#   container/push-images.sh arch PREFIX
#       push $IMAGE_NAME:pg$PG_MAJOR{,-cnpg,-postgres}-$ARCH (local, as built
#       with TAG_SUFFIX=-$ARCH) to $IMAGE_NAME:${PREFIX}pg$PG_MAJOR...-$ARCH
#   container/push-images.sh manifest PREFIX TAG-PREFIX...
#       join $IMAGE_NAME:${PREFIX}pg$PG_MAJOR...-{amd64,arm64} into one
#       manifest per variant, pushed as $IMAGE_NAME:<TAG-PREFIX>pg$PG_MAJOR...
#       for every TAG-PREFIX ("" for the moving release tags); prints
#       <repository>@<digest> per variant, one per line, for signing
#
#   IMAGE_NAME  repository, e.g. ghcr.io/evancarroll/pg-restrictive-usernames
#   PG_MAJOR    PostgreSQL major (default 18)
#   ARCH        this runner's architecture (default: buildah info)
#   ARCHES      architectures `manifest` joins (default "amd64 arm64")
#   TLS_VERIFY  false to talk to a plain-HTTP test registry (default true)
#
# The caller has run `buildah login` for the registry.
set -euo pipefail

: "${IMAGE_NAME:?IMAGE_NAME is required}"
PG_MAJOR="${PG_MAJOR:-18}"
ARCH="${ARCH:-$(buildah info --format '{{.host.arch}}')}"
ARCHES="${ARCHES:-amd64 arm64}"
TLS=(--tls-verify="${TLS_VERIFY:-true}")

# The CloudNativePG image exists for PostgreSQL 18 and later only
# (extension_control_path); build-extension.sh skips it below that.
variants=("" "-postgres")
if [ "$PG_MAJOR" -ge 18 ]; then variants+=("-cnpg"); fi

mode="${1:-}"
case "$mode" in
    arch)
        [ $# -eq 2 ] || { echo "usage: $0 arch PREFIX" >&2; exit 2; }
        prefix="$2"
        for v in "${variants[@]}"; do
            src="${IMAGE_NAME}:pg${PG_MAJOR}${v}-${ARCH}"
            dst="${IMAGE_NAME}:${prefix}pg${PG_MAJOR}${v}-${ARCH}"
            echo "==> ${src} -> ${dst}" >&2
            buildah push "${TLS[@]}" "$src" "docker://${dst}"
        done
        ;;
    manifest)
        [ $# -ge 3 ] || { echo "usage: $0 manifest PREFIX TAG-PREFIX..." >&2; exit 2; }
        prefix="$2"
        shift 2
        for v in "${variants[@]}"; do
            list="pru-pg${PG_MAJOR}${v}"
            buildah manifest rm "$list" >/dev/null 2>&1 || true
            buildah manifest create "$list" >/dev/null
            for a in $ARCHES; do
                buildah manifest add "${TLS[@]}" "$list" "docker://${IMAGE_NAME}:${prefix}pg${PG_MAJOR}${v}-${a}" >/dev/null
            done
            digest_file="$(mktemp)"
            first=1
            for t in "$@"; do
                dst="${IMAGE_NAME}:${t}pg${PG_MAJOR}${v}"
                echo "==> manifest ${dst}" >&2
                if [ "$first" = 1 ]; then
                    buildah manifest push "${TLS[@]}" --all --digestfile "$digest_file" "$list" "docker://${dst}"
                    first=0
                else
                    buildah manifest push "${TLS[@]}" --all "$list" "docker://${dst}"
                fi
            done
            echo "${IMAGE_NAME}@$(cat "$digest_file")"
            rm -f "$digest_file"
            buildah manifest rm "$list" >/dev/null
        done
        ;;
    *)
        sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//' >&2
        exit 2
        ;;
esac
