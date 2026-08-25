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

grep -F 'version = "0.1.0"' Cargo.toml >/dev/null
grep -F 'jlreq = { version = "0.1.0", path = "../jlreq" }' \
    crates/jlreq-conformance/Cargo.toml >/dev/null
if grep -F 'publish = false' crates/jlreq/Cargo.toml crates/jlreq-conformance/Cargo.toml; then
    echo "release crates must be publishable" >&2
    exit 1
fi
if grep -Eq '^git_(tag|release)_enable = true$' release-plz.toml; then
    echo "release-plz must remain externally inert during preparation" >&2
    exit 1
fi

git diff --exit-code -- crates/jlreq/src/generated data/manifest.toml \
    docs/generated/conformance-summary.md

echo "0.1.0 release state is internally consistent; no publication was performed"
