# SPDX-FileCopyrightText: 2026 jlreq contributors
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set shell := ["sh", "-cu"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

export RUSTDOCFLAGS := "-D warnings"

# The layout core must stay free of std, I/O, and font access (docs/adr/0001).
core_crates := "-p jlreq"

# Mutation testing targets the sole public library; xtask is repository tooling and the
# conformance product is an external black-box runner.
mutant_crates := "-p jlreq"

# A developer must be able to inspect an archive before committing; CI packages a clean
# checkout and therefore deliberately omits Cargo's dirty-tree escape hatch.
package_dirty := if env_var_or_default("CI", "") == "true" { "" } else { "--allow-dirty" }

# Where dune builds the reference engines in engines/, which is deliberately not its
# default `_build/` at the repository root. Dune copies every file a rule depends on into
# its build directory and the engines' rules depend on spec/captured/table*.tsv, so a
# default build leaves copies of the transcription outside spec/captured/ — where
# `just attest` reports them, because it confines the capture to the one directory a
# reviewer can read as one and skips only `.git` and `target` while it looks
# (docs/adr/0009). Building inside `target/` keeps the capture confined and the engines
# buildable at once. Dune takes a build directory outside the project root only as an
# absolute path, hence `justfile_directory()`.
dune_build_dir := "target" / "dune"
export DUNE_BUILD_DIR := justfile_directory() / dune_build_dir

# The engine executable the conformance runner is handed. `.exe` is dune's name for a
# native executable on every platform, Unix included.
ocaml_engine := dune_build_dir / "default/engines/ocaml/bin/jlreq_ocaml_engine.exe"

# Scratch space for the partial suite `ocaml-milestone` selects out of the built-in one.
# Nothing reads it but the run that just wrote it.
milestone_dir := "target" / "ocaml-milestone"

# The development probes (engines/ocaml/probe/). They are not the engine and no gate
# builds them: `diffcase` explains one case's difference field by field, and `census`
# generates the synthetic pair suites a milestone is debugged against.
diffcase_probe := dune_build_dir / "default/engines/ocaml/probe/diffcase.exe"
census_probe := dune_build_dir / "default/engines/ocaml/probe/census.exe"

# The Rust engine both probes compare against. `cargo build -p jlreq-conformance --bins`
# is what puts it here, and the recipes below run that first.
sample_engine := "target" / "debug/jlreq-sample-engine"

# Where a census run leaves its requests, both engines' answers, and the diff. Ignored
# with the rest of `target/`, and nothing but the next run of the same census reads it.
census_dir := "target" / "census"

# List the available development commands.
default:
    @just --list

# Format the workspace.
fmt:
    cargo fmt --all

# Check formatting without modifying files.
fmt-check:
    cargo fmt --all --check

# Check TOML formatting.
toml-check:
    taplo fmt --check --diff

# Run Clippy with and without default features across every target.
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo clippy --workspace --all-targets --no-default-features -- -D warnings

# Run the workspace suite. Nextest runs normal tests process-per-test; Cargo
# separately runs doctests, which nextest does not currently support.
test:
    cargo nextest run --workspace --all-features
    cargo test --workspace --doc --all-features

# Run the complete test suite with the non-fail-fast CI profile.
test-ci:
    cargo nextest run --profile ci --workspace --all-features
    cargo test --workspace --doc --all-features

# Build public documentation with warnings denied.
doc:
    cargo doc --workspace --all-features --no-deps

# Build and verify the public library archive, then inspect every file Cargo would put in the
# CLI archive. Cargo cannot create that second archive until its exact-version jlreq
# dependency has been published; the CLI's workspace build, tests, and MSRV run separately.
package:
    cargo package -p jlreq --locked {{package_dirty}}
    cargo package -p jlreq-conformance --locked {{package_dirty}} --list

# Compile no-default, every individual feature, and representative feature pairs.
feature-matrix:
    cargo hack check --workspace --all-targets --each-feature
    cargo hack check {{core_crates}} --feature-powerset --depth 2

# Prove that the layout core remains no_std. This is the mechanical half of
# docs/adr/0001; `just purity` is the declarative half.
no-std:
    rustup target add thumbv7em-none-eabi
    cargo build {{core_crates}} --target thumbv7em-none-eabi --no-default-features

# Prove the core builds for the browser, which no font- or IO-coupled layout
# engine can do (docs/adr/0003).
wasm:
    rustup target add wasm32-unknown-unknown
    cargo check {{core_crates}} --target wasm32-unknown-unknown --no-default-features

# Exercise malformed and extreme public inputs under libFuzzer and sanitizers. The target
# is a separate nightly workspace, so none of its dependencies enter the product graph.
# cargo-fuzz's MSVC runtime does not execute reliably; Windows still compiles the exact
# harness, while the required Linux CI job performs the bounded sanitizer run.
fuzz-check:
    {{ if os() == "windows" { "cargo +nightly check --manifest-path fuzz/Cargo.toml --bin public_api" } else { "cargo +nightly fuzz run public_api --fuzz-dir fuzz -- -runs=10000" } }}

# The install-action cargo-fuzz binary is itself built for musl. cargo-fuzz 0.13.2
# otherwise mistakes that build triple for the fuzz target, but ASan requires the
# dynamically linked GNU target used by GitHub's Ubuntu runner.
fuzz-check-linux-ci:
    cargo +nightly fuzz run public_api --fuzz-dir fuzz --target x86_64-unknown-linux-gnu -- -runs=10000

# Reject std, I/O, and font dependencies in the layout core (docs/adr/0001).
purity:
    cargo run --quiet -p xtask -- purity

# Reject unwritten bodies and suppressed lints in the layout core (CONTRIBUTING.md).
placeholder:
    cargo run --quiet -p xtask -- placeholder

# Hold jlreq to the exact 1.0 surface and all 22 typed Style mappings.
api:
    cargo run --quiet -p xtask -- api

# Require the private implementation modules to follow the one-way architecture in
# ARCHITECTURE.md.
direction:
    cargo run --quiet -p xtask -- direction

# Read the vendored specification snapshot into spec/derived/. Stage 1 of the pipeline
# (docs/design/generation.md).
derive:
    cargo run --quiet -p xtask -- derive

# Prove that rereading the snapshot would change no byte of spec/derived/. This is the
# gate; `just derive` is the writer.
derive-check:
    cargo run --quiet -p xtask -- derive --check

# Emit the specification data from spec/ (docs/design/generation.md).
generate:
    cargo run --quiet -p xtask -- generate

# Prove that regenerating the specification data would change no byte. This is the gate;
# `just generate` is the writer.
generate-check:
    cargo run --quiet -p xtask -- generate --check

# Attest the transcribed specification data against its double entry, its provenance, and
# the cross-table invariants (docs/adr/0009). `--digests` additionally hashes every recorded
# document present on disk, which is what anchors the derived files' digest chain to the
# upstream digests spec/PROVENANCE.toml records; the weaker form would leave that chain a
# closed loop agreeing with itself.
attest:
    cargo run --quiet -p xtask -- attest --digests

# Validate the conformance suite and the rule coverage it declares
# (docs/design/conformance.md).
conform:
    cargo run --quiet -p xtask -- conform --check

# Hold the unreleased 0.0.0 state, reject CRLF in tracked UTF-8 files, and reject broken
# local Markdown links (CONTRIBUTING.md).
repository:
    cargo run --quiet -p xtask -- repository

# Spell-check the repository.
typos:
    typos

# Check dependency advisories, bans, licenses, and sources.
deny:
    cargo deny --all-features check advisories bans licenses sources

# Reject unused, misplaced, and unlinked Cargo dependencies or source files.
shear:
    cargo shear --deny-warnings

# Check REUSE/SPDX compliance.
reuse:
    uvx --with charset-normalizer==3.4.9 reuse==6.2.0 lint

# Validate GitHub Actions workflows.
actionlint:
    actionlint -color

# Reject high-severity GitHub Actions and Dependabot security findings without
# granting the auditor network or repository credentials.
zizmor:
    zizmor --offline --persona regular --min-severity high .

# Verify every workspace crate at the shared declared MSRV.
msrv:
    cargo msrv verify --path crates/jlreq
    cargo msrv verify --path crates/jlreq-conformance
    cargo msrv verify --path xtask

# Mutation-test the crates with real logic against their own `#[cfg(test)]` suites, or one
# crate if `crate` is given (e.g. `just mutants jlreq`). It remains a scheduled report
# outside `ci-required` because a full mutation run is intentionally slow.
#
# `--test-tool nextest` matches `just test`. No `-D warnings` here unlike the other gates:
# `[workspace.lints]` sets these at `warn`, not `deny`, and CI only escalates them to errors
# by exporting `RUSTFLAGS` per job — which this recipe deliberately does not do. Mutated
# code that merely provokes a new lint (an unused binding, say) still builds and runs
# against the tests instead of being reported "unviable" for a reason unrelated to whether
# the tests actually exercise it. The unviable mutants this gate does report are almost all
# a different, structural thing: generic replacement values (`Default::default()`,
# `::std::iter::empty()`) that do not type-check against this crate's domain types or its
# `no_std` boundary — see the milestone report for the per-crate rate.
mutants crate="":
    cargo mutants {{ if crate == "" { mutant_crates } else { "-p " + crate } }} --test-tool nextest --no-times --colors=never -j 4

# Build the independent OCaml reference engine (engines/ocaml/README.md). The engines are
# outside the Cargo workspace and no Rust gate reads them, so the recipes below are the
# only things that build, test or run them. They are POSIX shell scripts; `conform-engines`
# is the one `just ci` calls and the one that knows about Windows.
#
# Dune creates DUNE_BUILD_DIR itself but refuses to create its parent, so a clean checkout
# with no `target/` yet (CI, before any cargo build has made one) fails before dune gets a
# chance to build anything. Creating it first makes this recipe work on its own.
ocaml-build:
    mkdir -p {{DUNE_BUILD_DIR}}
    dune build engines/ocaml

# Build the OCaml engine and run its own unit tests: the startup census of the
# specification tables, and the cross-check of the English transcription against the
# Japanese one this engine reads.
ocaml-test: ocaml-build
    dune runtest engines/ocaml

# Run the whole built-in suite against the OCaml engine. Before milestone M9 this is
# expected to report differences and exit 1 — a wrong answer is what the engine is still
# being written to fix. Exit 2 is different and is a real failure: it means the transport,
# the JSON or the specification tables are broken. `ocaml-gate` is what gates a merge, and
# at M9 it becomes this recipe by construction.
conform-ocaml: ocaml-build
    cargo run --quiet -p jlreq-conformance -- run {{ocaml_engine}}

# Run the cases of milestones M1 through M<m> against the OCaml engine: everything the
# engine claims to answer bit for bit, and nothing it does not. `m` is `0` until M1 lands,
# where no case is claimed yet and the unit tests are the whole gate.
ocaml-milestone m: ocaml-test
    {{ if m == "0" { "echo 'milestone 0: no conformance case is claimed yet'" } else { "just _ocaml-milestone " + m } }}

# The rest of `ocaml-milestone` for m >= 1, which needs more statements than the single
# conditional line of its caller can hold. Private because it is that recipe's body: the
# milestone files partition the suite, so the selection must find exactly as many cases as
# it named — an identifier matching nothing (a typo, or a case renamed in the suite) would
# otherwise shrink the gate without saying so.
_ocaml-milestone m:
    mkdir -p {{milestone_dir}}
    i=1; while [ "$i" -le {{m}} ]; do cat engines/ocaml/milestones/M$i.ids; i=$((i + 1)); done | sed -e 's/#.*$//' -e 's/[[:space:]]*$//' -e '/^$/d' -e 's|.*|"id":"&",|' > {{milestone_dir}}/ids.txt
    grep -F -f {{milestone_dir}}/ids.txt crates/jlreq-conformance/suite.ndjson > {{milestone_dir}}/suite.ndjson
    test "$(wc -l < {{milestone_dir}}/ids.txt)" -eq "$(wc -l < {{milestone_dir}}/suite.ndjson)" || { echo "a milestone identifier names no case in the built-in suite" >&2; exit 1; }
    cargo run --quiet -p jlreq-conformance -- run {{ocaml_engine}} {{milestone_dir}}/suite.ndjson

# The engine gate CI runs, and the one place that says how much of the suite the engine is
# held to today. engines/ocaml/milestones/CURRENT names that milestone, so advancing it is
# one digit in one file, reviewed in the pull request that earns it, and no workflow has to
# be edited to keep up.
ocaml-gate:
    just ocaml-milestone "$(sed -e 's/#.*$//' -e '/^[[:space:]]*$/d' engines/ocaml/milestones/CURRENT)"

# The engine gate as `just ci` runs it. It is deliberately not a hard dependency on an
# OCaml toolchain: a developer working on the Rust side has no opam switch, and a local
# gate that fails for that reason is a gate that gets routed around. The skip is loud, and
# the required conform-ocaml job in CI is what actually enforces it.
conform-engines:
    {{ if os() == "windows" { "if (Get-Command dune -ErrorAction SilentlyContinue) { just ocaml-gate } else { Write-Output 'SKIPPED conform-ocaml: no OCaml toolchain (CI enforces it)' }" } else { "if command -v dune > /dev/null 2>&1; then just ocaml-gate; else echo 'SKIPPED conform-ocaml: no OCaml toolchain (CI enforces it)'; fi" } }}

# Explain one conformance case's difference field by field, which `DIFF <case-id>` does
# not. The comparison is structural, so a key order is never reported and a missing key
# always is; the reference side is the suite's own `expected`, or `--rust` for the Rust
# sample engine's live answer on the same request.
#
#   just diffcase quick-start/two-lines
#   just diffcase quick-start/two-lines --rust
#   lines[1].clusters[0].inline: expected 0, got 250
#
# POSIX shell only, like every recipe in this block.
#
# Explain one case's difference by JSON path; exits 1 when the answers differ.
diffcase case *arguments: ocaml-build
    cargo build --quiet -p jlreq-conformance --bins
    {{diffcase_probe}} {{case}} --engine {{ocaml_engine}} --reference {{sample_engine}} --suite crates/jlreq-conformance/suite.ndjson {{arguments}}

# Generate one synthetic census, run it through both engines, and report how many answers
# disagree. `kind` is `spacing`, `break`, `reduction` or `expansion`; the registry that
# names them is `kinds` in `engines/ocaml/probe/census.ml` and nothing else knows the list.
#
#   just census spacing
#   census spacing: 2116 request(s), 2116 differing response(s) -- target/census/spacing.diff
#
# The two answer streams are canonicalized before `diff` sees them, because the engines
# write the same object with different key orders -- the Rust side's serde_json sorts the
# keys, the OCaml side writes `lines` before `diagnostics` -- and a raw textual diff would
# report every line as different. `diff` compares the Rust answer against the OCaml one,
# so a `<` line is what was expected and a `>` line is what this engine said.
#
# Generate one synthetic class-pair census and diff both engines' answers to it.
census kind: ocaml-build
    cargo build --quiet -p jlreq-conformance --bins
    mkdir -p {{census_dir}}
    {{census_probe}} generate {{kind}} > {{census_dir}}/{{kind}}.requests.ndjson
    {{ocaml_engine}} < {{census_dir}}/{{kind}}.requests.ndjson > {{census_dir}}/{{kind}}.ocaml.raw
    {{sample_engine}} < {{census_dir}}/{{kind}}.requests.ndjson > {{census_dir}}/{{kind}}.rust.raw
    {{census_probe}} normalize < {{census_dir}}/{{kind}}.ocaml.raw > {{census_dir}}/{{kind}}.ocaml.ndjson
    {{census_probe}} normalize < {{census_dir}}/{{kind}}.rust.raw > {{census_dir}}/{{kind}}.rust.ndjson
    diff {{census_dir}}/{{kind}}.rust.ndjson {{census_dir}}/{{kind}}.ocaml.ndjson > {{census_dir}}/{{kind}}.diff || true
    echo "census {{kind}}: $(wc -l < {{census_dir}}/{{kind}}.requests.ndjson | tr -d ' ') request(s), $(grep -c '^<' {{census_dir}}/{{kind}}.diff || true) differing response(s) -- {{census_dir}}/{{kind}}.diff"

# The choice is made out of Appendix A alone: fewest classes listing the key first, then a
# single scalar over a sequence, then an empty Remarks cell, then document order.
#
# Print the representative code point a census addresses each character class by, as TSV.
census-classes: ocaml-build
    {{census_probe}} classes

# The gates that hold the design itself, all of them reading the tree and none of them
# needing the network (docs/design/api-spine.md).
design: purity placeholder api direction derive-check generate-check attest conform repository
    @echo "design gates passed"

# Fast deterministic checks used during the edit/commit loop.
check: fmt-check toml-check typos lint design shear reuse actionlint zizmor
    @echo "fast local checks passed"

# Every practical CI gate available on a developer machine.
ci: fmt-check toml-check typos lint feature-matrix test-ci doc package no-std wasm fuzz-check design deny shear reuse actionlint zizmor msrv conform-engines
    @echo "local CI passed"
