#!/bin/sh
# SPDX-FileCopyrightText: 2026 jlreq contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 YYYY-MM-DD" >&2
    exit 2
fi

release_date=$1
case "$release_date" in
    [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]) ;;
    *)
        echo "finalize-release: date must be YYYY-MM-DD" >&2
        exit 2
        ;;
esac

if ! awk -v date="$release_date" '
BEGIN {
    split(date, part, "-")
    year = part[1] + 0
    month = part[2] + 0
    day = part[3] + 0
    leap = (year % 4 == 0 && (year % 100 != 0 || year % 400 == 0))
    days[1] = 31
    days[2] = leap ? 29 : 28
    days[3] = 31
    days[4] = 30
    days[5] = 31
    days[6] = 30
    days[7] = 31
    days[8] = 31
    days[9] = 30
    days[10] = 31
    days[11] = 30
    days[12] = 31
    exit !(year >= 2000 && month >= 1 && month <= 12 && day >= 1 && day <= days[month])
}
'; then
    echo "finalize-release: $release_date is not a valid calendar date" >&2
    exit 2
fi

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
changelog=$root/CHANGELOG.md
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)
test -n "$version" || {
    echo "finalize-release: could not read workspace version" >&2
    exit 2
}

unreleased_count=$(grep -Fxc '## [Unreleased]' "$changelog" || true)
if [ "$unreleased_count" -ne 1 ]; then
    echo "finalize-release: CHANGELOG.md must contain exactly one Unreleased heading" >&2
    exit 2
fi
if grep -Fq "## [$version]" "$changelog" || grep -Fq "[$version]:" "$changelog"; then
    echo "finalize-release: $version is already finalized in CHANGELOG.md" >&2
    exit 2
fi

scratch=$(mktemp "$root/CHANGELOG.md.finalize.XXXXXX")
cleanup() {
    rm -f "$scratch"
}
trap cleanup EXIT HUP INT TERM

awk -v version="$version" -v date="$release_date" '
$0 == "## [Unreleased]" {
    print
    print ""
    print "Nothing yet."
    print ""
    print "## [" version "] - " date
    skip_blank = 1
    next
}
skip_blank && $0 == "" {
    skip_blank = 0
    next
}
skip_blank {
    skip_blank = 0
}
$0 ~ /^\[Unreleased\]:/ {
    print "[Unreleased]: https://github.com/P4suta/jlreq/compare/v" version "...HEAD"
    print "[" version "]: https://github.com/P4suta/jlreq/releases/tag/v" version
    next
}
{
    print
}
' "$changelog" >"$scratch"

mv "$scratch" "$changelog"
trap - EXIT HUP INT TERM
echo "finalize-release: prepared $version for $release_date; review and commit CHANGELOG.md"
