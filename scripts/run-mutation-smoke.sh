#!/bin/sh

# SPDX-FileCopyrightText: 2026 jlreq contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 BASE_COMMIT" >&2
    exit 2
fi

base=$1
if ! base_commit=$(git rev-parse --verify --end-of-options "${base}^{commit}"); then
    echo "mutation smoke: base is not a commit: $base" >&2
    exit 2
fi

mkdir -p target
diff_file=target/mutants-smoke.diff
git diff --no-ext-diff --unified=0 --output="$diff_file" \
    "${base_commit}...HEAD" -- '*.rs'

if [ ! -s "$diff_file" ]; then
    echo "mutation smoke: no changed Rust lines"
    exit 0
fi

cargo mutants -p jlreq -p jlreq-core -p jlreq-conformance --in-diff "$diff_file" \
    --all-features \
    --test-tool cargo \
    --minimum-test-timeout 120 \
    --no-times \
    --colors=never \
    -j 4
