<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Release procedure

This guide separates reversible preparation from irreversible publication. A crates.io
version cannot be overwritten or deleted; yanking only prevents new resolutions. Nothing
in `just release-check` publishes, tags, chooses a date, creates a GitHub Release, changes
repository settings, or stores a credential.

## Prepared state

`just release-check` is the complete local acceptance command. It requires a clean tracked
tree and runs generation/attestation/design gates, tests and doctests, line/region coverage,
all mutation shards, fuzz compilation, MSRVs (facade 1.88, core 1.85), core `no_std`,
WASM, semver/API checks, dependency/license checks, package extraction, and the independent
engine census.

`just package` first performs `cargo fetch --locked` into an isolated Cargo home. It then
packages, extracts, builds, tests, documents, and where applicable installs these archives
offline:

1. `jlreq-core-0.1.0.crate`;
2. `jlreq-0.1.0.crate`; and
3. `jlreq-conformance-0.1.0.crate`.

The manual **Release check** workflow adds one CycloneDX SBOM and provenance attestation per
crate. Its six target jobs build archives for Linux x86_64 GNU/musl, Linux AArch64 GNU,
Windows x86_64 MSVC, and macOS x86_64/AArch64. Each target archive contains both
`jlreq-conformance` and `jlreq-sample-engine`, licenses, and README, with a target
checksum, SBOM, and attestations.

After downloading artifacts, verify every `.sha256` in its directory. Use
`gh attestation verify` against the repository for the corresponding archives and SBOMs.

## Repository settings before publication

The versioned policy is [`.github/REPOSITORY-SETTINGS.md`](../.github/REPOSITORY-SETTINGS.md).
A repository administrator must confirm:

- Actions are required to use full commit SHAs;
- the `release` environment permits deployment from `main` only, has at least one
  required reviewer, and disallows self-review;
- required CI, CodeQL, dependency review, REUSE, API, mutation, and Release Check jobs are
  green; and
- high/critical security alerts and open dependency-update PRs are zero.

Create the environment without a crate token. Reviewer identity and organization settings
cannot be encoded safely in this repository and must be recorded during the handoff.

## Finalize the candidate date

Only after a date is approved, run:

```sh
just finalize-release YYYY-MM-DD
```

The command validates an ISO calendar date, inserts `## [0.1.0] - YYYY-MM-DD` below a fresh
`[Unreleased]` heading, and writes the comparison links. Review and commit that change,
then run **Release check** on the exact full commit SHA. Do not use this command during
reversible preparation merely to invent a date.

Record the successful workflow run ID. Download its artifacts; verify all checksums,
attestations, package contents, and the candidate commit. Confirm that none of the three
names/version pairs or `v0.1.0` already exists.

## First publication

The first release uses the manual **Release** workflow in `initial-token` mode, protected
by the `release` environment and its exact confirmation phrase. Immediately before the
approved run, add a narrowly scoped crates.io token as environment secret
`CRATES_IO_TOKEN`. Do not add a repository-level token.

The workflow performs this irreversible sequence:

1. publish `jlreq-core` at the candidate version;
2. poll until that exact core version is visible in the registry index;
3. publish `jlreq`;
4. publish `jlreq-conformance`;
5. only after all three uploads succeed, create and push the annotated version tag; and
6. create the GitHub Release from the already attested artifacts.

The facade and conformance archive dependencies are patched to the extracted core only for
pre-publication offline verification; published manifests resolve the just-indexed
`jlreq-core`.

If an upload or polling step times out, inspect crates.io before retrying. The upload may
already be permanent. Never blindly rerun the workflow.

## Trusted Publishing after 0.1.0

After all first uploads succeed, configure a Trusted Publisher for each of `jlreq-core`,
`jlreq`, and `jlreq-conformance`, restricted to this repository,
`.github/workflows/release.yml`, and the `release` environment. Verify a later dry run or
release through the OIDC path, then remove `CRATES_IO_TOKEN` and revoke the initial token.

Future releases choose `trusted-publishing`; the pinned crates.io auth action exchanges
GitHub OIDC for a short-lived token. The tag and GitHub Release still occur only after every
crate publication succeeds.
