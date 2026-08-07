# SPDX-FileCopyrightText: 2026 kumihan contributors
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set shell := ["sh", "-cu"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

export RUSTDOCFLAGS := "-D warnings"

# The layout core must stay free of std, I/O, and font access (docs/adr/0001).
core_crates := "-p jlreq-unit -p jlreq-spec -p jlreq-class -p jlreq-spacing -p jlreq-line -p jlreq-inline -p jlreq"

# The crates with real logic to mutate today: jlreq-line, jlreq-inline and jlreq are still
# bootstrap stubs (ROADMAP.md M1/M3+), and xtask is repository tooling, not the product
# (docs/design/api-spine.md). Each crate's `src/generated/` tables are left in scope rather
# than excluded by glob: most (jlreq-spec, jlreq-spacing) are pure data with no function
# body for cargo-mutants to touch, and the few it finds elsewhere (jlreq-class's bitflag
# combinators) are cheap to run. Either way their correctness is `generate-check` and
# `attest`'s job, not this gate's.
mutant_crates := "-p jlreq-unit -p jlreq-spec -p jlreq-class -p jlreq-spacing"

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

# Reject std, I/O, and font dependencies in the layout core (docs/adr/0001).
purity:
    cargo run --quiet -p xtask -- purity

# Hold the listed types to the operator, constructor, and scalar-channel tables of
# docs/api-frozen.toml (docs/adr/0011, docs/adr/0012).
ops:
    cargo run --quiet -p xtask -- ops

# Reject unwritten bodies and suppressed lints in the layout core (CONTRIBUTING.md).
placeholder:
    cargo run --quiet -p xtask -- placeholder

# Hold the published surface to the shape frozen in docs/api-frozen.toml (docs/adr/0012).
api:
    cargo run --quiet -p xtask -- api

# Require every public item of the core to cite a specification address that resolves and
# is tested, as far as the inventory and the conformance cases exist (docs/adr/0013).
spec-links:
    cargo run --quiet -p xtask -- spec-links

# Require the rules that read the writing direction to be exactly the rules the inventory
# marks direction-conditional (docs/adr/0011).
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
    cargo msrv verify --path crates/jlreq-unit
    cargo msrv verify --path crates/jlreq-spec
    cargo msrv verify --path crates/jlreq-class
    cargo msrv verify --path crates/jlreq-spacing
    cargo msrv verify --path crates/jlreq-line
    cargo msrv verify --path crates/jlreq-inline
    cargo msrv verify --path crates/jlreq
    cargo msrv verify --path crates/jlreq-conform
    cargo msrv verify --path xtask

# Mutation-test the crates with real logic against their own `#[cfg(test)]` suites, or one
# crate if `crate` is given (e.g. `just mutants jlreq-spacing`). Baseline only today: M1-a
# stands the gate up with a report, not a kill-everything threshold — that discipline, and
# the independently-authored cases it needs, arrive with M1-b. Not part of `check`, `ci` or
# `design` yet for the same reason (see the CI workflow's own `mutants` job for where it
# does run: `workflow_dispatch` and a weekly schedule, outside `ci-required`).
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

# The gates that hold the design itself, all of them reading the tree and none of them
# needing the network (docs/design/api-spine.md).
design: purity ops placeholder api spec-links direction derive-check generate-check attest conform
    @echo "design gates passed"

# Fast deterministic checks used during the edit/commit loop.
check: fmt-check toml-check typos lint design shear reuse actionlint zizmor
    @echo "fast local checks passed"

# Every practical CI gate available on a developer machine.
ci: fmt-check toml-check typos lint feature-matrix test-ci doc no-std wasm design deny shear reuse actionlint zizmor msrv
    @echo "local CI passed"
