# SPDX-FileCopyrightText: 2026 jlreq contributors
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set shell := ["sh", "-cu"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

export RUSTDOCFLAGS := "-D warnings"

# The layout core must stay free of std, I/O, and font access (docs/adr/0001).
core_crates := "-p jlreq"

# Mutation testing covers both handwritten Rust products. Generated tables and repository
# tooling have independent generation/attestation gates.
mutant_crates := "-p jlreq -p jlreq-conformance"

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

# The independent Racket reference engine (engines/racket/README.md). `raco exe` writes a
# real executable rather than a script, which is the shape the runner needs on every
# platform: it starts the engine with `Command::new(path)` and no interpreter of its own.
# Both this path and the `compiled/` directories `raco make` writes are gitignored.
#
# `racket` and `raco` are deliberately not resolved here. The engines' toolchains are
# outside mise (the version of record is the RACKET_VERSION in .github/workflows/ci.yml
# and engines/racket/README.md), so a developer who has installed Racket puts it on PATH
# and one who has not gets the loud SKIPPED line from `conform-engines`.
racket_engine := "engines" / "racket/bin/jlreq-engine-racket"

# Scratch space for the partial suite `racket-milestone` selects, kept apart from the
# OCaml one so that running both engines' gates in one shell cannot interleave.
racket_milestone_dir := "target" / "racket-milestone"

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

# Generated, reviewable result of the exhaustive three-engine census.
census_summary := "docs/generated/conformance-summary.md"

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

# Run the workspace suite. Nextest runs ordinary harnessed tests process-per-test. Cargo
# separately runs the harness-free synthetic transport executable and doctests, neither of
# which nextest currently executes.
test:
    cargo nextest run --workspace --all-features
    cargo test -p jlreq --lib pipeline::tests::ten_thousand_cluster_standard_paragraph_stays_below_the_search_budget -- --ignored --exact
    cargo test --release -p jlreq --lib pipeline::tests::zero_width_pathological_paragraph_stops_at_the_default_search_budget -- --ignored --exact
    cargo test -p jlreq-conformance --test transport --all-features
    cargo test --workspace --doc --all-features

# Run the complete test suite with the non-fail-fast CI profile.
test-ci:
    cargo nextest run --profile ci --workspace --all-features
    cargo test -p jlreq --lib pipeline::tests::ten_thousand_cluster_standard_paragraph_stays_below_the_search_budget -- --ignored --exact
    cargo test --release -p jlreq --lib pipeline::tests::zero_width_pathological_paragraph_stops_at_the_default_search_budget -- --ignored --exact
    cargo test -p jlreq-conformance --test transport --all-features
    cargo test --workspace --doc --all-features

# Build public documentation with warnings denied.
doc:
    cargo doc --workspace --all-features --no-deps

# Build and verify both public crate archives. The temporary crates.io patch lets Cargo verify
# the exact-version inter-crate dependency before jlreq has actually been uploaded.
package:
    cargo package -p jlreq --locked {{package_dirty}}
    cargo package -p jlreq-conformance --locked {{package_dirty}} --offline --config 'patch.crates-io.jlreq.path="crates/jlreq"'
    sh scripts/verify-crates.sh

# Ask Cargo to execute its complete crates.io publication preflight while retaining the
# upload locally. The patch validates jlreq-conformance before the first jlreq release has
# appeared in the index; published metadata still carries only version 0.1.0.
publish-dry-run:
    cargo publish --dry-run --locked {{package_dirty}} -p jlreq
    cargo publish --dry-run --locked {{package_dirty}} -p jlreq-conformance --config 'patch.crates-io.jlreq.path="crates/jlreq"'

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

# Exercise input validation, composition/arithmetic, and protocol parsing separately under
# libFuzzer. Curated seeds are copied below target/ so a run never dirties the source tree.
fuzz-check:
    {{ if os() == "windows" { "cargo +nightly check --manifest-path fuzz/Cargo.toml --bins" } else { "just _fuzz-target input_validation 30" } }}
    {{ if os() == "windows" { "cargo +nightly check --manifest-path fuzz/Cargo.toml --bins" } else { "just _fuzz-target composition 30" } }}
    {{ if os() == "windows" { "cargo +nightly check --manifest-path fuzz/Cargo.toml --bins" } else { "just _fuzz-target protocol_parser 30" } }}

# The install-action cargo-fuzz binary is itself built for musl. cargo-fuzz 0.13.2
# otherwise mistakes that build triple for the fuzz target, but ASan requires the
# dynamically linked GNU target used by GitHub's Ubuntu runner.
fuzz-check-linux-ci:
    just _fuzz-target-linux input_validation 30
    just _fuzz-target-linux composition 30
    just _fuzz-target-linux protocol_parser 30

# A single bounded fuzz target. Runtime corpora are disposable target/ state; only
# fuzz/seeds is reviewed and committed.
[private]
_fuzz-target target seconds:
    mkdir -p target/fuzz-corpus/{{target}}
    cp fuzz/seeds/{{target}}/* target/fuzz-corpus/{{target}}/
    cargo +nightly fuzz run {{target}} target/fuzz-corpus/{{target}} --fuzz-dir fuzz -- -max_total_time={{seconds}} -timeout=10

[private]
_fuzz-target-linux target seconds:
    mkdir -p target/fuzz-corpus/{{target}}
    cp fuzz/seeds/{{target}}/* target/fuzz-corpus/{{target}}/
    cargo +nightly fuzz run {{target}} target/fuzz-corpus/{{target}} --fuzz-dir fuzz --target x86_64-unknown-linux-gnu -- -max_total_time={{seconds}} -timeout=10

# Scheduled sanitizer budget: fifteen minutes for each independent failure domain.
fuzz-scheduled:
    just _fuzz-target-linux input_validation 900
    just _fuzz-target-linux composition 900
    just _fuzz-target-linux protocol_parser 900

# Each handwritten product must independently stay above both release thresholds. Generated
# tables, test fixtures, xtask, and independent engines are covered by their own gates. The
# transport regression deliberately kills its stalled synthetic engine, so LLVM may see that
# one incomplete profile; `all` still rejects a run in which no valid profile can be merged.
coverage:
    cargo llvm-cov -p jlreq --all-features --ignore-filename-regex '(/src/generated/|/tests/)' --fail-under-lines 90 --fail-under-regions 85 --summary-only
    cargo llvm-cov -p jlreq-conformance --all-features --exclude-from-report jlreq --ignore-filename-regex '(/tests/)' --failure-mode all --fail-under-lines 90 --fail-under-regions 85 --summary-only

# Reject std, I/O, and font dependencies in the layout core (docs/adr/0001).
purity:
    cargo run --quiet -p xtask -- purity

# Reject unwritten bodies and suppressed lints in the layout core (CONTRIBUTING.md).
placeholder:
    cargo run --quiet -p xtask -- placeholder

# Hold jlreq to the exact 0.1.0 export surface and all 22 typed Style mappings.
api:
    cargo run --quiet -p xtask -- api

# Before the initial release, hold the local 0.1.0 control in both directions. For every
# later 0.1.x candidate, additionally compare the complete rustdoc API with the latest
# published jlreq release and reject patch-incompatible changes.
semver:
    sh scripts/check-semver.sh

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

# Hold the prepared 0.1.0 state without performing publication, reject CRLF in tracked UTF-8
# files, and reject broken local Markdown links (CONTRIBUTING.md).
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
    uvx --with charset-normalizer==3.4.9 reuse==6.2.0 --no-multiprocessing lint

# Validate GitHub Actions workflows.
actionlint:
    actionlint -color

# Validate every repository-owned POSIX shell entry point, including release packaging and
# the three-engine census driver.
shellcheck:
    shellcheck engines/census-all.sh scripts/*.sh

# Reject high-severity GitHub Actions and Dependabot security findings without
# granting the auditor network or repository credentials.
zizmor:
    zizmor --offline --persona regular --min-severity high .

# Verify every workspace crate at the shared declared MSRV.
msrv:
    cargo msrv verify --path crates/jlreq
    cargo msrv verify --path crates/jlreq-conformance
    cargo msrv verify --path xtask

# Mutation-test both handwritten products, or one package/shard when supplied. Generated
# table and exact equivalent-mutant exclusions are pinned in docs/mutation-ledger.toml. Any
# missed or timed-out mutant makes cargo-mutants, and therefore this gate, fail.
#
# Cargo's test tool is intentional: unlike nextest it also runs the harness-free synthetic
# transport executable. No `-D warnings` here unlike the other gates:
# `[workspace.lints]` sets these at `warn`, not `deny`, and CI only escalates them to errors
# by exporting `RUSTFLAGS` per job — which this recipe deliberately does not do. Mutated
# code that merely provokes a new lint (an unused binding, say) still builds and runs
# against the tests instead of being reported "unviable" for a reason unrelated to whether
# the tests actually exercise it. The unviable mutants this gate does report are almost all
# a different, structural thing: generic replacement values (`Default::default()`,
# `::std::iter::empty()`) that do not type-check against this crate's domain types or its
# `no_std` boundary — see the milestone report for the per-crate rate.
mutants crate="" shard="":
    sh scripts/verify-mutation-ledger.sh
    cargo mutants {{ if crate == "" { mutant_crates } else { "-p " + crate } }} {{ if shard == "" { "" } else { "--shard " + shard } }} --test-tool cargo --minimum-test-timeout 120 --no-times --colors=never -j 4

# Pull requests exercise only mutations in the changed Rust surface; weekly and release
# workflows run the complete sharded gate above.
mutants-smoke base:
    sh scripts/verify-mutation-ledger.sh
    cargo mutants --workspace --in-diff {{base}} --test-tool cargo --minimum-test-timeout 120 --no-times --colors=never -j 4

# Hold generated and equivalent-mutant exclusions to their individual source hashes and
# require every cargo-mutants regex to have one proof in the reviewable ledger.
mutation-ledger:
    sh scripts/verify-mutation-ledger.sh

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

# Build the independent Racket reference engine (engines/racket/README.md).
#
# `raco make` compiles the module graph to bytecode and `raco exe` embeds it in an
# executable. The specification tables are pasted into the bytecode at compile time
# rather than opened at run time (engines/racket/embed.rkt), so the result is the
# argument-free, path-free executable docs/design/conformance.md's runner contract
# requires — `raco distribute` is deliberately not in the chain, because there is no
# runtime-path file left for it to gather.
build-engine-racket:
    mkdir -p engines/racket/bin
    raco make engines/racket/main.rkt
    raco exe -o {{racket_engine}} engines/racket/main.rkt

# Run the Racket engine's own unit tests: the integer contract, the TSV reader, the
# envelope, the startup census of the specification tables, and the cross-check of the
# Japanese transcription against the English one this engine reads.
#
# The tests directory and not the whole engine: `raco test` on a directory runs every
# module it finds, and running `main.rkt` means starting the NDJSON loop on this
# terminal's stdin.
test-engine-racket:
    raco test engines/racket/tests

# Run the whole built-in suite against the Racket engine. Before milestone M9 this is
# expected to report differences and exit 1 — a wrong answer is what the engine is still
# being written to fix. Exit 2 is different and is a real failure: it means the transport,
# the JSON or the specification tables are broken. `racket-gate` is what gates a merge,
# and at M9 it becomes this recipe by construction.
conform-racket: build-engine-racket
    cargo run --quiet -p jlreq-conformance -- run {{racket_engine}}

# Run the cases of milestones M1 through M<m> against the Racket engine: everything the
# engine claims to answer bit for bit, and nothing it does not. `m` is `0` until M1 lands,
# where no case is claimed yet and the unit tests are the whole gate.
racket-milestone m: build-engine-racket test-engine-racket
    {{ if m == "0" { "echo 'milestone 0: no conformance case is claimed yet'" } else { "just _racket-milestone " + m } }}

# The rest of `racket-milestone` for m >= 1, which needs more statements than the single
# conditional line of its caller can hold. Private because it is that recipe's body: the
# milestone files partition the suite, so the selection must find exactly as many cases as
# it named — an identifier matching nothing (a typo, or a case renamed in the suite) would
# otherwise shrink the gate without saying so.
_racket-milestone m:
    mkdir -p {{racket_milestone_dir}}
    i=1; while [ "$i" -le {{m}} ]; do cat engines/racket/milestones/M$i.ids; i=$((i + 1)); done | sed -e 's/#.*$//' -e 's/[[:space:]]*$//' -e '/^$/d' -e 's|.*|"id":"&",|' > {{racket_milestone_dir}}/ids.txt
    grep -F -f {{racket_milestone_dir}}/ids.txt crates/jlreq-conformance/suite.ndjson > {{racket_milestone_dir}}/suite.ndjson
    test "$(wc -l < {{racket_milestone_dir}}/ids.txt)" -eq "$(wc -l < {{racket_milestone_dir}}/suite.ndjson)" || { echo "a milestone identifier names no case in the built-in suite" >&2; exit 1; }
    cargo run --quiet -p jlreq-conformance -- run {{racket_engine}} {{racket_milestone_dir}}/suite.ndjson

# The Racket engine gate CI runs, and the one place that says how much of the suite that
# engine is held to today. engines/racket/milestones/CURRENT names that milestone, so
# advancing it is one digit in one file, reviewed in the pull request that earns it, and
# no workflow has to be edited to keep up.
racket-gate:
    just racket-milestone "$(sed -e 's/#.*$//' -e '/^[[:space:]]*$/d' engines/racket/milestones/CURRENT)"

# Both engine gates as `just ci` runs them. Neither is a hard dependency on its toolchain:
# a developer working on the Rust side has no opam switch and no Racket, and a local gate
# that fails for that reason is a gate that gets routed around. Each skip is loud, and the
# required conform-ocaml and conform-racket jobs in CI are what actually enforce them.
conform-engines:
    {{ if os() == "windows" { "if (Get-Command dune -ErrorAction SilentlyContinue) { just ocaml-gate } else { Write-Output 'SKIPPED conform-ocaml: no OCaml toolchain (CI enforces it)' }" } else { "if command -v dune > /dev/null 2>&1; then just ocaml-gate; else echo 'SKIPPED conform-ocaml: no OCaml toolchain (CI enforces it)'; fi" } }}
    {{ if os() == "windows" { "if (Get-Command raco -ErrorAction SilentlyContinue) { just racket-gate } else { Write-Output 'SKIPPED conform-racket: no Racket toolchain (CI enforces it)' }" } else { "if command -v raco > /dev/null 2>&1; then just racket-gate; else echo 'SKIPPED conform-racket: no Racket toolchain (CI enforces it)'; fi" } }}

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
# disagree. `kind` is `spacing`, `break`, `reduction`, `expansion`, `vertical` or
# `tate-chu-yoko`; the registry that names them is `kinds` in
# `engines/ocaml/probe/census.ml` and nothing else knows the list.
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

# The same census, against the Racket engine.
#
#   just census-racket spacing
#   census-racket spacing: 2116 request(s), 0 differing response(s) -- target/census/spacing.racket.diff
#
# The generator and the canonicalizer are `engines/ocaml/probe/census.ml`, reused rather
# than reimplemented: a census is a stream of requests and a normalizer for the answers,
# neither of which is an implementation of JLReq, so writing a second one would test two
# probes against each other instead of two engines. The independence rule is about the
# layout logic (docs/design/conformance.md), and this borrows none of it — which is why
# this recipe needs both toolchains while `conform-racket` needs only Racket.
#
# `diff` compares the Rust answer against the Racket one, so a `<` line is what was
# expected and a `>` line is what this engine said. Both streams are canonicalized first,
# because key order is not part of an answer.
census-racket kind: build-engine-racket ocaml-build
    cargo build --quiet -p jlreq-conformance --bins
    mkdir -p {{census_dir}}
    {{census_probe}} generate {{kind}} > {{census_dir}}/{{kind}}.requests.ndjson
    {{racket_engine}} < {{census_dir}}/{{kind}}.requests.ndjson > {{census_dir}}/{{kind}}.racket.raw
    {{sample_engine}} < {{census_dir}}/{{kind}}.requests.ndjson > {{census_dir}}/{{kind}}.rust.raw
    {{census_probe}} normalize < {{census_dir}}/{{kind}}.racket.raw > {{census_dir}}/{{kind}}.racket.ndjson
    {{census_probe}} normalize < {{census_dir}}/{{kind}}.rust.raw > {{census_dir}}/{{kind}}.rust.ndjson
    diff {{census_dir}}/{{kind}}.rust.ndjson {{census_dir}}/{{kind}}.racket.ndjson > {{census_dir}}/{{kind}}.racket.diff || true
    echo "census-racket {{kind}}: $(wc -l < {{census_dir}}/{{kind}}.requests.ndjson | tr -d ' ') request(s), $(grep -c '^<' {{census_dir}}/{{kind}}.racket.diff || true) differing response(s) -- {{census_dir}}/{{kind}}.racket.diff"

# Run every census in the generator registry through Rust, OCaml, and Racket. The script
# verifies all three pairings, the response cardinality, the ten-kind registry, the
# 122,199-case floor, and the committed generated summary. Any difference is fatal.
census-all: ocaml-test build-engine-racket test-engine-racket
    cargo build --quiet -p jlreq-conformance --bins
    sh engines/census-all.sh {{census_probe}} {{sample_engine}} {{ocaml_engine}} {{racket_engine}} {{census_summary}}

# The gates that hold the design itself, all of them reading the tree and none of them
# needing the network (docs/design/api-spine.md).
design: purity placeholder api direction derive-check generate-check attest conform mutation-ledger repository
    @echo "design gates passed"

# Fast deterministic checks used during the edit/commit loop.
check: fmt-check toml-check typos lint design shear reuse shellcheck actionlint zizmor
    @echo "fast local checks passed"

# Every practical CI gate available on a developer machine.
ci: fmt-check toml-check typos lint feature-matrix test-ci doc package no-std wasm fuzz-check coverage design semver deny shear reuse shellcheck actionlint zizmor msrv conform-engines
    @echo "local CI passed"

# Release acceptance performs no publication, tag, GitHub Release, or external settings
# change. It intentionally requires a clean tracked tree so generated and packaged output
# is reproducible from the candidate commit.
release-check:
    test -z "$(git status --porcelain --untracked-files=no)" || { echo "release-check requires a clean tracked tree" >&2; exit 1; }
    just ci
    just publish-dry-run
    just census-all
    just mutants jlreq
    just mutants jlreq-conformance
    sh scripts/verify-release-state.sh
