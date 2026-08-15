# SPDX-FileCopyrightText: 2026 kumihan contributors
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set shell := ["sh", "-cu"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

export RUSTDOCFLAGS := "-D warnings"

# The layout core must stay free of std, I/O, and font access (docs/adr/0001).
core_crates := "-p kumihan"

# Mutation testing targets the sole public library; xtask is repository tooling and the
# conformance product is an external black-box runner.
mutant_crates := "-p kumihan"

# A developer must be able to inspect an archive before committing; CI packages a clean
# checkout and therefore deliberately omits Cargo's dirty-tree escape hatch.
package_dirty := if env_var_or_default("CI", "") == "true" { "" } else { "--allow-dirty" }

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
# CLI archive. Cargo cannot create that second archive until its exact-version kumihan
# dependency has been published; the CLI's workspace build, tests, and MSRV run separately.
package:
    cargo package -p kumihan --locked {{package_dirty}}
    cargo package -p kumihan-conformance --locked {{package_dirty}} --list

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

# Hold kumihan to the exact 1.0 surface and all 22 typed Style mappings.
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
    cargo msrv verify --path crates/kumihan
    cargo msrv verify --path crates/kumihan-conformance
    cargo msrv verify --path xtask

# Mutation-test the crates with real logic against their own `#[cfg(test)]` suites, or one
# crate if `crate` is given (e.g. `just mutants kumihan`). It remains a scheduled report
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

# The gates that hold the design itself, all of them reading the tree and none of them
# needing the network (docs/design/api-spine.md).
design: purity placeholder api direction derive-check generate-check attest conform repository
    @echo "design gates passed"

# Fast deterministic checks used during the edit/commit loop.
check: fmt-check toml-check typos lint design shear reuse actionlint zizmor
    @echo "fast local checks passed"

# Every practical CI gate available on a developer machine.
ci: fmt-check toml-check typos lint feature-matrix test-ci doc package no-std wasm fuzz-check design deny shear reuse actionlint zizmor msrv
    @echo "local CI passed"
