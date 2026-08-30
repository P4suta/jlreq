#!/bin/sh
# SPDX-FileCopyrightText: 2026 jlreq contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

test -f docs/public-api.toml
test ! -e docs/api-1.0.toml
test -f docs/generated/conformance-summary.md
test -f docs/error-codes.md
test -f docs/mutation-ledger.toml
test -f .github/workflows/release.yml
test -f .github/workflows/release-check.yml
test -f .github/REPOSITORY-SETTINGS.md
test -f LICENSE-MIT
test -f LICENSE-APACHE

grep -F 'version = "0.1.0"' Cargo.toml >/dev/null
grep -F 'jlreq-core = { version = "0.1.0", path = "../jlreq-core" }' \
    crates/jlreq/Cargo.toml >/dev/null
grep -F 'jlreq-core = { version = "0.1.0", path = "../jlreq-core" }' \
    crates/jlreq-conformance/Cargo.toml >/dev/null
if grep -F 'publish = false' crates/jlreq/Cargo.toml crates/jlreq-core/Cargo.toml \
    crates/jlreq-conformance/Cargo.toml; then
    echo "release crates must be publishable" >&2
    exit 1
fi

for package in jlreq-core jlreq jlreq-conformance; do
    test -f "target/dist/$package-0.1.0.crate" || {
        echo "missing verified prerelease archive for $package" >&2
        exit 1
    }
done
(
    cd target/dist
    manifest=jlreq-0.1.0-crates.sha256
    expected=$(printf '%s\n' \
        jlreq-core-0.1.0.crate \
        jlreq-0.1.0.crate \
        jlreq-conformance-0.1.0.crate | sort)
    actual=$(awk 'NF { name = $2; sub(/^\*/, "", name); print name }' "$manifest" | sort)
    if [ "$actual" != "$expected" ]; then
        echo "$manifest must contain exactly one entry for each release crate and no others" >&2
        exit 1
    fi
    sha256sum --check --strict "$manifest"
)
test -z "$(git tag --list v0.1.0)" || {
    echo "v0.1.0 tag must not exist during prerelease preparation" >&2
    exit 1
}
if grep -Eq '^git_(tag|release)_enable = true$' release-plz.toml; then
    echo "release-plz must remain externally inert during preparation" >&2
    exit 1
fi

git diff --exit-code -- crates/jlreq-core/src/generated data/manifest.toml \
    docs/generated/conformance-summary.md

echo "0.1.0 release state is internally consistent; no publication was performed"
