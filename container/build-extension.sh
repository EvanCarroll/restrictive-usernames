#!/usr/bin/env bash
# Build (and optionally test) the pg_restrictive_usernames extension inside a
# buildah builder, and commit up to three images of it:
#
#   container/build-extension.sh [--test] [--out DIR]
#
#   --test      run `cargo pgrx test` against PostgreSQL $PG_MAJOR first
#   --out DIR   also copy the packaged tree to DIR on the host
#   PG_MAJOR        PostgreSQL major version, 14-18 (default 18)
#   IMAGE_NAME      repository of the committed images (default localhost/pg-restrictive-usernames)
#   TAG_SUFFIX      appended to every tag, e.g. -amd64 (default none)
#   IMAGE_SOURCE    org.opencontainers.image.source label (default file://<checkout>)
#   IMAGE_REVISION  org.opencontainers.image.revision label (default git HEAD[-dirty], if any)
#
# $IMAGE_NAME:pg$PG_MAJOR, FROM scratch, holds the packaged extension at
#   /opt/pg-restrictive-usernames/pg$PG_MAJOR/usr/lib/postgresql/$PG_MAJOR/lib/pg_restrictive_usernames.so
#   /opt/pg-restrictive-usernames/pg$PG_MAJOR/usr/share/postgresql/$PG_MAJOR/extension/pg_restrictive_usernames.control
#   /opt/pg-restrictive-usernames/pg$PG_MAJOR/usr/share/postgresql/$PG_MAJOR/extension/pg_restrictive_usernames--<version>.sql
#   /opt/pg-restrictive-usernames/pg$PG_MAJOR/LICENSE, LICENSE.restrictive-usernames
# for a database image build to `buildah copy --from`.
#
# $IMAGE_NAME:pg$PG_MAJOR-cnpg (PostgreSQL 18 and later), FROM scratch, is a
# CloudNativePG image volume extension: CNPG mounts it at
# /extensions/<name>/ and appends share/ to extension_control_path and lib/
# to dynamic_library_path.
#   /lib/pg_restrictive_usernames.so
#   /share/extension/pg_restrictive_usernames.control, pg_restrictive_usernames--*.sql
#   /licenses/pg_restrictive_usernames/LICENSE, LICENSE.restrictive-usernames
#
# $IMAGE_NAME:pg$PG_MAJOR-postgres is docker.io/library/postgres:$PG_MAJOR-trixie
# with the extension installed, to try it:
#   podman run -d -e POSTGRES_PASSWORD=x IMAGE
#
# The FROM-scratch images have no shell: inspect one with
#   podman create --name x IMAGE true && podman export x | tar -t; podman rm x
set -euo pipefail

PG_MAJOR="${PG_MAJOR:-18}"
RUN_TESTS=0
OUT_DIR=""

# The header comment through the option list.
usage() { sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
    case "$1" in
        --test) RUN_TESTS=1; shift ;;
        --out)
            [ $# -ge 2 ] || { echo "--out needs a directory" >&2; exit 2; }
            OUT_DIR="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
    esac
done

case "$PG_MAJOR" in
    14|15|16|17|18) ;;
    *) echo "PG_MAJOR must be 14-18 (one pgrx feature each), not ${PG_MAJOR}" >&2; exit 2 ;;
esac

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE_NAME="${IMAGE_NAME:-localhost/pg-restrictive-usernames}"
TAG_SUFFIX="${TAG_SUFFIX:-}"
# Tags carry no architecture by default. Each build is for the host
# architecture only (pgrx links against the target's PostgreSQL headers and
# cargo pgrx test runs a cluster, both slow or fragile under emulation), so
# CI builds on one runner per architecture with TAG_SUFFIX=-<arch>, pushes
# those, and joins them with `buildah manifest` under the suffix-less tag.
# Either way the image config records the architecture (set explicitly on
# the scratch images, inherited from the host-arch base on the try-it
# image), which is what a manifest list indexes by.
ARCH="$(buildah info --format '{{.host.arch}}')"
IMAGE="${IMAGE_NAME}:pg${PG_MAJOR}${TAG_SUFFIX}"
IMAGE_CNPG="${IMAGE_NAME}:pg${PG_MAJOR}-cnpg${TAG_SUFFIX}"
IMAGE_POSTGRES="${IMAGE_NAME}:pg${PG_MAJOR}-postgres${TAG_SUFFIX}"
# The builder's Debian release. The .so links against its glibc, so the
# PGDG packages, the try-it base image and the CNPG os label all follow it.
DISTRO=trixie
PG_CONFIG="/usr/lib/postgresql/${PG_MAJOR}/bin/pg_config"
DEST="/opt/pg-restrictive-usernames/pg${PG_MAJOR}"
EXT=pg_restrictive_usernames
FEATURES="--no-default-features --features pg${PG_MAJOR}"

# Unprivileged build user: initdb refuses to run as root, and keeping every
# cargo step under one user means one ~/.pgrx config and one target dir.
BUILD_HOME=/home/build
BUILD_PATH="${BUILD_HOME}/.cargo/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

# Policy and Unicode versions for the image labels, read from the library
# source (the same constants restrictive_usernames.version() and
# unicode_version() return).
LIB_RS="${REPO}/crates/restrictive-usernames/src/lib.rs"
POLICY_VERSION="$(sed -n 's/^pub const POLICY_VERSION: &str = "\(.*\)";$/\1/p' "$LIB_RS")"
# rustfmt puts UNICODE_VERSION on one line or wraps the string to the next,
# depending on its length; accept both.
UNICODE_VERSION="$(sed -n \
    -e 's/^pub const UNICODE_VERSION: &str = "\(.*\)";$/\1/p' \
    -e '/^pub const UNICODE_VERSION: &str =$/{n;s/^ *"\(.*\)";$/\1/p}' "$LIB_RS")"
[ -n "$POLICY_VERSION" ] || { echo "cannot read POLICY_VERSION from $LIB_RS" >&2; exit 1; }
[ -n "$UNICODE_VERSION" ] || { echo "cannot read UNICODE_VERSION from $LIB_RS" >&2; exit 1; }
# org.opencontainers.image.source: CI passes the repository URL; a local
# build names the checkout it was built from.
IMAGE_SOURCE="${IMAGE_SOURCE:-file://${REPO}}"
# org.opencontainers.image.revision: the commit, marked -dirty when tracked
# files differ from it (the build stages the working tree, not HEAD).
if [ -z "${IMAGE_REVISION:-}" ] && IMAGE_REVISION="$(git -C "$REPO" rev-parse HEAD 2>/dev/null)"; then
    if [ -n "$(git -C "$REPO" status --porcelain --untracked-files=no)" ]; then
        IMAGE_REVISION="${IMAGE_REVISION}-dirty"
    fi
fi
IMAGE_REVISION="${IMAGE_REVISION:-}"

STAGE="$(mktemp -d)"
ctr=""
img=""
cleanup() {
    rm -rf "$STAGE"
    if [ -n "$img" ]; then buildah rm "$img" >/dev/null || true; fi
    if [ -n "$ctr" ]; then buildah rm "$ctr" >/dev/null || true; fi
}
trap cleanup EXIT

# Stage the source without build output, VCS data or a local lockfile (CI
# has none either; dependencies float).
tar -C "$REPO" --exclude=./target --exclude=./out --exclude=./.git --exclude=./Cargo.lock -cf - . \
    | tar -C "$STAGE" -xf -

ctr="$(buildah from docker.io/library/rust:1-${DISTRO})"

as_root() { buildah run "$ctr" -- bash -euo pipefail -c "$1"; }
as_build() {
    buildah run --user build \
        --env HOME="$BUILD_HOME" \
        --env USER=build \
        --env CARGO_HOME="${BUILD_HOME}/.cargo" \
        --env PATH="$BUILD_PATH" \
        --workingdir /src/crates/${EXT} \
        "$ctr" -- bash -euo pipefail -c "$1"
}

echo "==> PostgreSQL ${PG_MAJOR} from apt.postgresql.org (${DISTRO}-pgdg)"
as_root "
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y --no-install-recommends ca-certificates curl jq
    install -d /usr/share/postgresql-common/pgdg
    curl -fsSL --retry 3 --retry-all-errors -o /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc \
        https://www.postgresql.org/media/keys/ACCC4CF8.asc
    echo 'deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] https://apt.postgresql.org/pub/repos/apt ${DISTRO}-pgdg main' \
        > /etc/apt/sources.list.d/pgdg.list
    apt-get update
    apt-get install -y --no-install-recommends \
        postgresql-${PG_MAJOR} postgresql-server-dev-${PG_MAJOR} \
        clang libclang-dev pkg-config libssl-dev
    rm -rf /var/lib/apt/lists/*
    useradd -m -d ${BUILD_HOME} build
    install -d -o build /out
"

buildah copy --chown build:build "$ctr" "$STAGE" /src >/dev/null

echo "==> rust toolchain"
as_build "rustc --version; cargo --version"

# cargo-pgrx must equal the pgrx crate version exactly: read it from the
# resolved dependency graph instead of hard-coding it.
PGRX_VERSION="$(as_build "cargo metadata --format-version 1 | jq -er '[.packages[] | select(.name == \"pgrx\") | .version] | unique | if length == 1 then .[0] else error(\"pgrx resolved to \\(.)\") end'")"
echo "==> cargo-pgrx ${PGRX_VERSION}"
as_build "cargo install --locked cargo-pgrx@${PGRX_VERSION}"
as_build "cargo pgrx init --pg${PG_MAJOR} ${PG_CONFIG}"

if [ "$RUN_TESTS" = 1 ]; then
    echo "==> cargo pgrx test pg${PG_MAJOR}"
    # cargo pgrx test installs the extension into the system PostgreSQL
    # before starting a throwaway cluster; let the build user write there.
    as_root "chown build \$(${PG_CONFIG} --pkglibdir) \$(${PG_CONFIG} --sharedir)/extension"
    as_build "cargo pgrx test ${FEATURES} pg${PG_MAJOR}"
    as_build "cargo pgrx schema --pg-config ${PG_CONFIG} ${FEATURES} --out /out/schema.sql pg${PG_MAJOR}"
fi

echo "==> cargo pgrx package"
as_build "cargo pgrx package --pg-config ${PG_CONFIG} ${FEATURES} --out-dir /out/package"

as_root "
    lib=/out/package\$(${PG_CONFIG} --pkglibdir)
    share=/out/package\$(${PG_CONFIG} --sharedir)/extension
    test -f \$lib/${EXT}.so
    test -f \$share/${EXT}.control
    ls \$share/${EXT}--*.sql >/dev/null
    install -d ${DEST}
    cp -a /out/package/. ${DEST}/
    # The extension is under the PostgreSQL License; the .so statically
    # links restrictive-usernames, which is MIT. Ship both notices.
    install -m 0644 /src/crates/${EXT}/LICENSE ${DEST}/LICENSE
    install -m 0644 /src/LICENSE ${DEST}/LICENSE.restrictive-usernames
    # Owned by root: a consumer's buildah copy --from keeps ownership.
    chown -R root:root /opt/pg-restrictive-usernames
    # The generated schema, for the record. (That every function lands in
    # schema restrictive_usernames is asserted against pg_proc by a pg_test.)
    if [ -f /out/schema.sql ]; then
        install -m 0644 /out/schema.sql ${DEST}/schema.sql
    fi
"

# Debian's PGDG layout, the same in the builder and in postgres:*-${DISTRO}.
PKGLIB="$(as_root "${PG_CONFIG} --pkglibdir")"
SHARE="$(as_root "${PG_CONFIG} --sharedir")/extension"

EXT_VERSION="$(as_build "cargo metadata --format-version 1 --no-deps | jq -er '.packages[] | select(.name == \"${EXT}\") | .version'")"

# Labels on every image. org.opencontainers.image.version is the extension
# version; the policy and Unicode versions are what decide whether a REINDEX
# is due, so they are labelled too.
common_labels=(
    --label org.opencontainers.image.source="${IMAGE_SOURCE}"
    --label org.opencontainers.image.url="${IMAGE_SOURCE}"
    --label org.opencontainers.image.version="${EXT_VERSION}"
    --label org.opencontainers.image.authors="Evan Carroll <me@evancarroll.com>"
    --label pg-restrictive-usernames.pg-major="${PG_MAJOR}"
    --label pg-restrictive-usernames.policy-version="${POLICY_VERSION}"
    --label pg-restrictive-usernames.unicode-version="${UNICODE_VERSION}"
)
if [ -n "$IMAGE_REVISION" ]; then
    common_labels+=(--label org.opencontainers.image.revision="${IMAGE_REVISION}")
fi
# What the FROM-scratch images contain: the extension (PostgreSQL License)
# and the library linked into it (MIT).
scratch_labels=(
    --label org.opencontainers.image.licenses="PostgreSQL AND MIT"
    --label org.opencontainers.image.base.name=scratch
)

# The builder is an intermediate. The tagged image is FROM scratch and holds
# only ${DEST} (the two usr/ subtrees, the licenses and schema.sql), at the
# same path, for a database image build to buildah copy --from.
img="$(buildah from scratch)"
buildah copy --from "$ctr" "$img" "$DEST" "$DEST" >/dev/null
buildah config --os linux --arch "$ARCH" \
    "${common_labels[@]}" "${scratch_labels[@]}" \
    --label org.opencontainers.image.title=pg-restrictive-usernames \
    --label org.opencontainers.image.description="pg_restrictive_usernames ${EXT_VERSION} packaged for PostgreSQL ${PG_MAJOR} under ${DEST}" \
    "$img"
buildah commit --rm "$img" "$IMAGE" >/dev/null
img=""
echo "==> committed ${IMAGE} (from scratch, ${ARCH})"

# CloudNativePG image volume extension. extension_control_path and the
# matching dynamic_library_path search are new in PostgreSQL 18, so older
# majors get no such image. Layout, labels and the non-root user follow
# github.com/cloudnative-pg/postgres-extensions-containers (e.g. pgvector):
# /lib and /share/extension at the root, licenses under /licenses/<name>/.
# io.cloudnativepg.image.base.name names the CNPG operand image whose
# PostgreSQL and Debian release (PGDG postgresql-${PG_MAJOR} on ${DISTRO})
# this was built against; a Cluster must run that major, OS and arch.
if [ "$PG_MAJOR" -ge 18 ]; then
    as_root "
        root=/out/cnpg
        rm -rf \$root
        install -d \$root/lib \$root/share/extension \$root/licenses/${EXT}
        cp -a ${DEST}${PKGLIB}/${EXT}.so \$root/lib/
        cp -a ${DEST}${SHARE}/${EXT}.control ${DEST}${SHARE}/${EXT}--*.sql \$root/share/extension/
        cp -a ${DEST}/LICENSE ${DEST}/LICENSE.restrictive-usernames \$root/licenses/${EXT}/
        chown -R root:root \$root
    "
    img="$(buildah from scratch)"
    buildah copy --from "$ctr" "$img" /out/cnpg / >/dev/null
    buildah config --os linux --arch "$ARCH" --user 65532:65532 \
        "${common_labels[@]}" "${scratch_labels[@]}" \
        --label org.opencontainers.image.title="pg_restrictive_usernames ${EXT_VERSION} ${PG_MAJOR} ${DISTRO}" \
        --label org.opencontainers.image.description="A pg_restrictive_usernames ${EXT_VERSION} extension image for CloudNativePG, PostgreSQL ${PG_MAJOR} on ${DISTRO}" \
        --label io.cloudnativepg.image.base.name="ghcr.io/cloudnative-pg/postgresql:${PG_MAJOR}-minimal-${DISTRO}" \
        --label io.cloudnativepg.image.base.pgmajor="${PG_MAJOR}" \
        --label io.cloudnativepg.image.base.os="${DISTRO}" \
        --label io.cloudnativepg.image.sql.version="${EXT_VERSION}" \
        "$img"
    buildah commit --rm "$img" "$IMAGE_CNPG" >/dev/null
    img=""
    echo "==> committed ${IMAGE_CNPG} (from scratch, ${ARCH})"
else
    echo "==> no CloudNativePG image for PostgreSQL ${PG_MAJOR} (extension_control_path is 18+)"
fi

# Try-it image: the official postgres image of the same major and Debian
# release as the builder (same glibc), with the packaged files where its
# pg_config says. Not for production: build a database image from the
# scratch image instead.
img="$(buildah from docker.io/library/postgres:${PG_MAJOR}-${DISTRO})"
buildah copy --from "$ctr" "$img" "${DEST}${PKGLIB}/" "${PKGLIB}/" >/dev/null
buildah copy --from "$ctr" "$img" "${DEST}${SHARE}/" "${SHARE}/" >/dev/null
buildah copy --from "$ctr" "$img" "${DEST}/LICENSE" "${DEST}/LICENSE.restrictive-usernames" "/usr/share/doc/${EXT}/" >/dev/null
# The base image's pg_config must agree with the builder's, or the files
# above landed where its server does not look.
buildah run "$img" -- bash -c "test \"\$(pg_config --pkglibdir)\" = ${PKGLIB} && test \"\$(pg_config --sharedir)/extension\" = ${SHARE}"
buildah config \
    "${common_labels[@]}" \
    --label org.opencontainers.image.title=pg-restrictive-usernames-postgres \
    --label org.opencontainers.image.description="postgres:${PG_MAJOR}-${DISTRO} with pg_restrictive_usernames ${EXT_VERSION}" \
    --label org.opencontainers.image.base.name="docker.io/library/postgres:${PG_MAJOR}-${DISTRO}" \
    "$img"
buildah commit --rm "$img" "$IMAGE_POSTGRES" >/dev/null
img=""
echo "==> committed ${IMAGE_POSTGRES} (postgres:${PG_MAJOR}-${DISTRO}, ${ARCH})"

if [ -n "$OUT_DIR" ]; then
    mkdir -p "$OUT_DIR"
    buildah run "$ctr" -- tar -C "$DEST" -cf - . | tar -C "$OUT_DIR" -xf -
    echo "==> copied to ${OUT_DIR}"
fi

echo "==> packaged files"
buildah run "$ctr" -- find "$DEST" -type f
