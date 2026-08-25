#!/bin/sh
# SPDX-FileCopyrightText: 2026 jlreq contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
version=0.1.0
package_dir=$root/target/package
audit=$(mktemp -d "${TMPDIR:-/tmp}/jlreq-crates.XXXXXX")
trap 'rm -r "$audit"' EXIT HUP INT TERM

for package in jlreq jlreq-conformance; do
    archive=$package_dir/$package-$version.crate
    test -f "$archive" || {
        echo "missing crate archive: $archive" >&2
        exit 1
    }
    size=$(wc -c < "$archive" | tr -d ' ')
    test "$size" -le 5242880 || {
        echo "$archive exceeds the 5 MiB release limit ($size bytes)" >&2
        exit 1
    }
    if tar -tzf "$archive" | grep -Eq '(^/|(^|/)\.\.(/|$))'; then
        echo "$archive contains an unsafe path" >&2
        exit 1
    fi
    for required in Cargo.toml README.md LICENSE-MIT LICENSE-APACHE; do
        tar -tzf "$archive" | grep -Fx "$package-$version/$required" >/dev/null || {
            echo "$archive omits $required" >&2
            exit 1
        }
    done
    tar -xzf "$archive" -C "$audit"
done

for required in protocol.schema.json suite.ndjson; do
    test -f "$audit/jlreq-conformance-$version/$required" || {
        echo "jlreq-conformance archive omits $required" >&2
        exit 1
    }
done

cargo test --manifest-path "$audit/jlreq-$version/Cargo.toml" --all-targets --offline
cargo test --manifest-path "$audit/jlreq-$version/Cargo.toml" --doc --offline

patch="patch.crates-io.jlreq.path=\"$audit/jlreq-$version\""
cargo test --manifest-path "$audit/jlreq-conformance-$version/Cargo.toml" --all-targets \
    --offline --config "$patch"
# A binary-only package has no Cargo doctest target. Building its rustdoc proves that the
# package documentation target itself is complete and valid in the extracted archive.
cargo doc --manifest-path "$audit/jlreq-conformance-$version/Cargo.toml" --no-deps \
    --offline --config "$patch"
cargo install --path "$audit/jlreq-conformance-$version" --root "$audit/install" \
    --locked --offline --config "$patch"

suffix=
case $(uname -s) in
    MINGW*|MSYS*|CYGWIN*) suffix=.exe ;;
esac
"$audit/install/bin/jlreq-conformance$suffix" --version | grep -Fx \
    "jlreq-conformance $version" >/dev/null
"$audit/install/bin/jlreq-conformance$suffix" --help >/dev/null
"$audit/install/bin/jlreq-sample-engine$suffix" --help >/dev/null 2>&1 || {
    status=$?
    test "$status" -eq 2
}

echo "crate archives verified: jlreq $version and jlreq-conformance $version"
