#!/usr/bin/env sh
# SPDX-FileCopyrightText: 2026 jlreq contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)
test -n "$version" || {
    echo "could not read workspace version" >&2
    exit 1
}

audit=$(mktemp -d "${TMPDIR:-/tmp}/jlreq-crates.XXXXXX")
trap 'rm -rf "$audit"' EXIT HUP INT TERM
mkdir -p "$audit/cargo-home" "$audit/install"

# Populate a brand-new Cargo home from the frozen workspace lockfile before any archive is
# opened. Every package check below is offline, so a passing run cannot borrow the developer's
# registry/index cache or hide a newly introduced dependency.
CARGO_HOME="$audit/cargo-home" cargo fetch --locked --manifest-path "$root/Cargo.toml"

for package in jlreq-core jlreq jlreq-conformance; do
    archive="$root/target/package/$package-$version.crate"
    test -f "$archive" || {
        echo "missing package archive: $archive" >&2
        exit 1
    }
    tar -xzf "$archive" -C "$audit"
    for required in Cargo.toml Cargo.toml.orig README.md LICENSE-MIT LICENSE-APACHE; do
        test -f "$audit/$package-$version/$required" || {
            echo "$package archive omits $required" >&2
            exit 1
        }
    done
done

core="$audit/jlreq-core-$version"
facade="$audit/jlreq-$version"
conformance="$audit/jlreq-conformance-$version"
core_patch="patch.crates-io.jlreq-core.path=\"$core\""

CARGO_HOME="$audit/cargo-home" cargo test --manifest-path "$core/Cargo.toml" \
    --all-targets --offline
CARGO_HOME="$audit/cargo-home" cargo test --manifest-path "$core/Cargo.toml" \
    --doc --offline

CARGO_HOME="$audit/cargo-home" cargo test --manifest-path "$facade/Cargo.toml" \
    --all-targets --all-features --offline --config "$core_patch"
CARGO_HOME="$audit/cargo-home" cargo test --manifest-path "$facade/Cargo.toml" \
    --doc --all-features --offline --config "$core_patch"
CARGO_HOME="$audit/cargo-home" cargo doc --manifest-path "$facade/Cargo.toml" \
    --no-deps --all-features --offline --config "$core_patch"

CARGO_HOME="$audit/cargo-home" cargo test --manifest-path "$conformance/Cargo.toml" \
    --all-targets --all-features --offline --config "$core_patch"
CARGO_HOME="$audit/cargo-home" cargo doc --manifest-path "$conformance/Cargo.toml" \
    --no-deps --all-features --offline --config "$core_patch"
CARGO_HOME="$audit/cargo-home" cargo install --path "$conformance" --root "$audit/install" \
    --locked --offline --config "$core_patch"

suffix=
if [ "${OS:-}" = Windows_NT ]; then
    suffix=.exe
fi
"$audit/install/bin/jlreq-conformance$suffix" --version | grep -Fx \
    "jlreq-conformance $version" >/dev/null
"$audit/install/bin/jlreq-conformance$suffix" --help >/dev/null
"$audit/install/bin/jlreq-sample-engine$suffix" --help >/dev/null 2>&1 || {
    echo "installed sample engine did not accept --help" >&2
    exit 1
}

dist="$root/target/dist"
mkdir -p "$dist"
for package in jlreq-core jlreq jlreq-conformance; do
    cp "$root/target/package/$package-$version.crate" "$dist/"
done
(
    cd "$dist"
    sha256sum \
        "jlreq-core-$version.crate" \
        "jlreq-$version.crate" \
        "jlreq-conformance-$version.crate" >"jlreq-$version-crates.sha256"
    sha256sum --check --strict "jlreq-$version-crates.sha256"
)

echo "crate archives verified offline from an isolated Cargo home: jlreq-core, jlreq, jlreq-conformance $version"
