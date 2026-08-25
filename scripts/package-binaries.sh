#!/bin/sh
# SPDX-FileCopyrightText: 2026 jlreq contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: package-binaries.sh RUST-TARGET" >&2
    exit 2
fi

target=$1
case $target in
    '' | *[!A-Za-z0-9._-]*)
        echo "RUST-TARGET contains an unsafe path character: $target" >&2
        exit 2
        ;;
esac

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
version=0.1.0
name=jlreq-$version-$target
dist=$root/target/dist

mkdir -p "$dist"
scratch=$(mktemp -d "$dist/.package-$target.XXXXXX")
stage=$scratch/$name
mkdir -p "$stage"

cleanup() {
    rm -r "$scratch"
}
trap cleanup EXIT HUP INT TERM

cargo build --locked --release --target "$target" -p jlreq-conformance --bins

suffix=
case $target in
    *windows*) suffix=.exe ;;
esac

for binary in jlreq-conformance jlreq-sample-engine; do
    cp "$root/target/$target/release/$binary$suffix" "$stage/"
done
cp "$root/README.md" "$stage/"
cp "$root/LICENSES/MIT.txt" "$stage/LICENSE-MIT"
cp "$root/LICENSES/Apache-2.0.txt" "$stage/LICENSE-APACHE"

case $target in
    *windows*)
        archive=$dist/$name.zip
        candidate=$scratch/$name.zip
        (cd "$scratch" && 7z a -bd -tzip "$candidate" "$name" >/dev/null)
        ;;
    *)
        archive=$dist/$name.tar.gz
        candidate=$scratch/$name.tar.gz
        tar -czf "$candidate" -C "$scratch" "$name"
        ;;
esac

case $candidate in
    *.zip)
        listing=$(7z l -ba "$candidate")
        printf '%s\n' "$listing" | grep -F "$name/jlreq-conformance$suffix" >/dev/null
        printf '%s\n' "$listing" | grep -F "$name/jlreq-sample-engine$suffix" >/dev/null
        printf '%s\n' "$listing" | grep -F "$name/README.md" >/dev/null
        printf '%s\n' "$listing" | grep -F "$name/LICENSE-MIT" >/dev/null
        printf '%s\n' "$listing" | grep -F "$name/LICENSE-APACHE" >/dev/null
        ;;
    *)
        for required in jlreq-conformance jlreq-sample-engine README.md LICENSE-MIT LICENSE-APACHE; do
            tar -tzf "$candidate" | grep -Fx "$name/$required" >/dev/null
        done
        ;;
esac

size=$(wc -c < "$candidate" | tr -d ' ')
test "$size" -le 52428800 || {
    echo "$candidate exceeds the 50 MiB binary-archive limit" >&2
    exit 1
}

mv -f "$candidate" "$archive"
if command -v sha256sum >/dev/null 2>&1; then
    (cd "$dist" && sha256sum "$(basename "$archive")" > "$(basename "$archive").sha256")
else
    (cd "$dist" && shasum -a 256 "$(basename "$archive")" > "$(basename "$archive").sha256")
fi

echo "$archive"
