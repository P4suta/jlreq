<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Release procedure

This guide separates reversible preparation from irreversible publication. Cargo documents
that a published version is permanent: it cannot be overwritten or deleted, and yanking
only prevents new resolutions. Always inspect `cargo package`/`cargo publish --dry-run`
output first ([Cargo publishing reference](https://doc.rust-lang.org/cargo/reference/publishing.html)).

## Prepared state

`just release-check` is the single non-publishing acceptance command. It requires a clean
tracked tree and verifies generation/attestation/design gates, tests and doctests, coverage,
full mutation runs, all three reference engines and the 122,199-case census, MSRV 1.85,
`no_std`, WASM, the public API contract, package contents, extracted crate builds, and CLI
installation. At 0.1.0 the semver gate fixes the local baseline without contacting a
registry; later 0.1.x candidates are checked in patch mode against the latest published
jlreq release. All repository-owned shell entry points are checked by ShellCheck 0.11.0.
The manual
`Release check` workflow runs that command and builds the six target archives:

- Linux x86_64 GNU and musl, and Linux AArch64 GNU;
- Windows x86_64 MSVC; and
- macOS x86_64 and AArch64.

`just publish-dry-run` is part of that command and contacts crates.io for Cargo's complete
publication preflight, but Cargo retains both uploads locally. The conformance dry-run uses
the packaged local `jlreq` only because 0.1.0 is not in the index before the first release.

The mutation step first verifies [the exclusion ledger](mutation-ledger.toml): all ten
generated table files have individual hashes, and each of the five proven-equivalent
mutants has one exact regex, source hash, and proof. Handwritten integrity checks are not
excluded.

Each archive contains `jlreq-conformance`, `jlreq-sample-engine`, the repository README,
and both license texts. The workflow emits SHA-256 files and target-scoped CycloneDX 1.5
JSON, validates its component, version, license, and target triple, and records GitHub
build-provenance and SBOM attestations. Verify downloaded artifacts with `sha256sum -c`;
after publication, consumers can additionally use `gh attestation verify`.

## First publication checklist

The first crates.io release cannot use Trusted Publishing. crates.io requires one manual
release before a Trusted Publisher can be configured
([Rust announcement](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/)).
The `Release` workflow therefore has an `initial-token` mode protected by the `release`
GitHub environment and an exact confirmation phrase. It is intentionally never triggered
by a push or tag.

Before approving it:

- [ ] Replace the `Unreleased` heading with `0.1.0` and the approved date; update the
      comparison link.
- [ ] Confirm the candidate commit passed `Release check` and record that workflow run ID.
- [ ] Download its artifacts, verify every checksum and attestation, and inspect both
      `.crate` archives.
- [ ] Confirm the `jlreq` and `jlreq-conformance` names and the 0.1.0 version have not
      already been published.
- [ ] Store a narrowly scoped crates.io token in the `release` environment as
      `CRATES_IO_TOKEN`, require environment approval, and enter the exact workflow
      confirmation phrase.

The workflow performs the irreversible sequence in this order:

1. publish `jlreq` 0.1.0;
2. wait until that exact version is visible in the crates.io index;
3. publish `jlreq-conformance` 0.1.0;
4. create and push `v0.1.0`; and
5. create the GitHub Release from the already attested archives, checksums, SBOMs, and
   `.crate` files.

If Cargo times out while polling the index, check crates.io before retrying: the upload may
already be permanent. Never rerun a publish blindly.

## Trusted Publishing after 0.1.0

After both first uploads succeed, configure each crate's Trusted Publisher for this
repository, `.github/workflows/release.yml`, and the `release` environment. Future releases
select `trusted-publishing`; the official
[`rust-lang/crates-io-auth-action`](https://github.com/rust-lang/crates-io-auth-action)
exchanges GitHub OIDC for a short-lived crates.io token and revokes it after the job.
Remove the long-lived `CRATES_IO_TOKEN` secret after verifying the OIDC path.

Tag creation, GitHub Release creation, Trusted Publisher configuration, branch-protection
changes, and the two uploads are external mutations. None is part of `just release-check`.
