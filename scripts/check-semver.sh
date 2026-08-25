#!/bin/sh

# SPDX-FileCopyrightText: 2026 jlreq contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -eu

control=docs/public-api.toml

contract_value() {
    key=$1
    values=$(sed -n "s/^${key} = \"\([^\"]*\)\"$/\\1/p" "$control")
    lines=$(printf '%s\n' "$values" | awk 'NF { count += 1 } END { print count + 0 }')
    if [ "$lines" -ne 1 ]; then
        echo "semver: $control must contain exactly one $key string" >&2
        exit 2
    fi
    printf '%s\n' "$values"
}

baseline_version=$(contract_value baseline_version)
compatible_series=$(contract_value compatible_series)
package_id=$(cargo pkgid -p jlreq)
current_version=${package_id##*#}
current_version=${current_version##*@}

case "$baseline_version" in
    "$compatible_series".*) ;;
    *)
        echo "semver: baseline $baseline_version is outside compatible series $compatible_series" >&2
        exit 2
        ;;
esac

case "$current_version" in
    "$baseline_version" | "$compatible_series".*) ;;
    *)
        echo "semver: jlreq $current_version is outside the $compatible_series.x contract in $control" >&2
        echo "semver: review and update the release-line policy before continuing" >&2
        exit 2
        ;;
esac

# This local, network-free control is always enforced, including before the first version
# exists in a registry. It detects missing and extra exports and policy-choice drift.
cargo run --quiet -p xtask -- api

# Before 0.1.0 is published there is no external semantic-versioning baseline. A caller can
# supply a source baseline to exercise the complete check in tests or release engineering.
if [ -n "${JLREQ_SEMVER_BASELINE_ROOT:-}" ]; then
    if [ ! -d "$JLREQ_SEMVER_BASELINE_ROOT" ] && [ ! -f "$JLREQ_SEMVER_BASELINE_ROOT" ]; then
        echo "semver: baseline root does not exist: $JLREQ_SEMVER_BASELINE_ROOT" >&2
        exit 2
    fi
    # The current tree's documentation warnings are already denied by `just doc`. Do not
    # make a later compiler's new warning in an immutable registry baseline masquerade as
    # a semantic-versioning failure.
    RUSTDOCFLAGS='' cargo semver-checks check-release \
        --manifest-path crates/jlreq/Cargo.toml \
        --package jlreq \
        --baseline-root "$JLREQ_SEMVER_BASELINE_ROOT" \
        --release-type patch \
        --all-features
    exit 0
fi

if [ "$current_version" = "$baseline_version" ]; then
    echo "semver: $current_version is the initial baseline; registry comparison begins with the next $compatible_series.x candidate"
    exit 0
fi

# For later 0.1.x candidates cargo-semver-checks resolves the latest normal, non-yanked
# crates.io version. Comparing with the latest release also protects API added in an
# intermediate patch release, not only the original 0.1.0 surface.
RUSTDOCFLAGS='' cargo semver-checks check-release \
    --manifest-path crates/jlreq/Cargo.toml \
    --package jlreq \
    --release-type patch \
    --all-features
