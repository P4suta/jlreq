#!/bin/sh
# SPDX-FileCopyrightText: 2026 jlreq contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -eu

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

ledger=docs/mutation-ledger.toml
config=.cargo/mutants.toml
scratch=$(mktemp -d "${TMPDIR:-/tmp}/jlreq-mutation-ledger.XXXXXX")

cleanup() {
    rm -r "$scratch"
}
trap cleanup EXIT HUP INT TERM

fail() {
    echo "mutation-ledger: $*" >&2
    exit 1
}

digest() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -- "$1" | awk '{ print $1 }'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 -- "$1" | awk '{ print $1 }'
    else
        fail "neither sha256sum nor shasum is available"
    fi
}

# The ledger deliberately uses only flat, quoted fields inside these array tables. Parse
# that closed shape instead of adding a package dependency merely to validate repository
# metadata.
awk '
function clear() {
    path = sha = kind = provenance = reason = ""
}
function value(line) {
    sub(/^[^=]+=[[:space:]]*/, "", line)
    return substr(line, 2, length(line) - 2)
}
function emit() {
    if (section != "exclusion") return
    if (path == "" || sha == "" || kind == "" || provenance == "" || reason == "") {
        print "docs/mutation-ledger.toml: incomplete [[exclusion]]" > "/dev/stderr"
        failed = 1
        return
    }
    print path "\t" sha "\t" kind "\t" provenance "\t" reason
}
$0 == "[[exclusion]]" {
    emit()
    section = "exclusion"
    clear()
    next
}
/^\[\[/ {
    emit()
    section = ""
    clear()
    next
}
section == "exclusion" && /^path = / { path = value($0); next }
section == "exclusion" && /^sha256 = / { sha = value($0); next }
section == "exclusion" && /^kind = / { kind = value($0); next }
section == "exclusion" && /^provenance = / { provenance = value($0); next }
section == "exclusion" && /^reason = / { reason = value($0); next }
END {
    emit()
    if (failed) exit 1
}
' "$ledger" >"$scratch/exclusions.tsv"

awk '
function clear() {
    mutant = pattern = sha = proof = ""
}
function value(line) {
    sub(/^[^=]+=[[:space:]]*/, "", line)
    return substr(line, 2, length(line) - 2)
}
function emit() {
    if (section != "equivalent") return
    if (mutant == "" || pattern == "" || sha == "" || proof == "") {
        print "docs/mutation-ledger.toml: incomplete [[equivalent]]" > "/dev/stderr"
        failed = 1
        return
    }
    print mutant "\t" pattern "\t" sha "\t" proof
}
$0 == "[[equivalent]]" {
    emit()
    section = "equivalent"
    clear()
    next
}
/^\[\[/ {
    emit()
    section = ""
    clear()
    next
}
section == "equivalent" && /^mutant = / { mutant = value($0); next }
section == "equivalent" && /^exclude_re = / { pattern = value($0); next }
section == "equivalent" && /^source_sha256 = / { sha = value($0); next }
section == "equivalent" && /^proof = / { proof = value($0); next }
END {
    emit()
    if (failed) exit 1
}
' "$ledger" >"$scratch/equivalent.tsv"

if grep -Eq '^glob = ' "$ledger"; then
    fail "generated exclusions must name individual paths, not a broad glob"
fi

find crates/jlreq/src/generated -maxdepth 1 -type f -name '*.rs' -print |
    LC_ALL=C sort >"$scratch/generated-files.txt"
cut -f 1 "$scratch/exclusions.tsv" | LC_ALL=C sort >"$scratch/excluded-files.txt"
if ! cmp -s "$scratch/generated-files.txt" "$scratch/excluded-files.txt"; then
    diff -u "$scratch/generated-files.txt" "$scratch/excluded-files.txt" >&2 || true
    fail "the ledger must exclude every generated table, and only those tables"
fi

duplicates=$(cut -f 1 "$scratch/exclusions.tsv" | LC_ALL=C sort | uniq -d)
test -z "$duplicates" || fail "duplicate generated exclusion: $duplicates"

tab=$(printf '\t')
while IFS="$tab" read -r path expected kind provenance reason; do
    test "$kind" = generated || fail "$path has non-generated exclusion kind $kind"
    test "$provenance" = data/manifest.toml || fail "$path is not anchored to data/manifest.toml"
    test -n "$reason" || fail "$path has no exclusion reason"
    test -f "$path" || fail "$path does not exist"
    test "${#expected}" -eq 64 || fail "$path has a malformed SHA-256"
    case "$expected" in
        *[!0-9a-f]*) fail "$path has a non-hexadecimal SHA-256" ;;
    esac
    actual=$(digest "$path")
    test "$actual" = "$expected" || fail "$path changed: expected $expected, observed $actual"
done <"$scratch/exclusions.tsv"

if ! grep -Fqx 'exclude_globs = ["crates/jlreq/src/generated/**"]' "$config"; then
    fail "$config must exclude the generated table directory exactly"
fi
if grep -Fq '"crates/jlreq/src/generated.rs"' "$config"; then
    fail "$config excludes the handwritten generated.rs integrity checks"
fi

duplicates=$(cut -f 1 "$scratch/equivalent.tsv" | LC_ALL=C sort | uniq -d)
test -z "$duplicates" || fail "duplicate equivalent mutant: $duplicates"
duplicates=$(cut -f 2 "$scratch/equivalent.tsv" | LC_ALL=C sort | uniq -d)
test -z "$duplicates" || fail "duplicate equivalent-mutant regex: $duplicates"

while IFS="$tab" read -r mutant pattern expected proof; do
    case "$mutant" in
        *.rs:*) source="${mutant%%.rs:*}.rs" ;;
        *) fail "equivalent mutant does not start with a Rust source path: $mutant" ;;
    esac
    test -n "$proof" || fail "$mutant has no equivalence proof"
    test -f "$source" || fail "$source does not exist"
    actual=$(digest "$source")
    test "$actual" = "$expected" || fail "$mutant is stale: expected $expected, observed $actual"
    grep -Fqx "  '$pattern'," "$config" || fail "$mutant has no exact regex in $config"
done <"$scratch/equivalent.tsv"

documented=$(wc -l <"$scratch/equivalent.tsv" | tr -d ' ')
configured=$(grep -c "^  '" "$config" || true)
test "$configured" -eq "$documented" ||
    fail "$config has $configured equivalent regex(es), but the ledger documents $documented"

generated=$(wc -l <"$scratch/exclusions.tsv" | tr -d ' ')
echo "mutation-ledger: verified $generated generated file(s) and $documented equivalent mutant(s) against source SHA-256"
