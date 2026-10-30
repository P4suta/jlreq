# Repository settings required for release

GitHub stores these controls outside the workflow source.
A repository administrator must verify the controls for the candidate and record the reviewer and verification date in the release handoff.
No crate credential belongs in these settings during preparation.

## Actions and branch protection

Pin every third-party GitHub Action to a full 40-character commit SHA.
Keep the platform setting enabled to enforce immutable action references.

Protect `main` with the `required` status from GitHub Actions.
That status depends on `ci-required`, which requires the Linux checks, Windows and macOS tests, and independent OCaml and Racket checks to succeed.
Require the branch to be current before merge, signed commits, linear history, pull requests, and resolved conversations.
Keep required checks protected from bypass.

Before approving a release candidate, verify its successful Release check run and CodeQL results.
Release acceptance includes dependency policy, license checks through `just reuse`, API compatibility, coverage, package verification, all five fuzz targets, and full mutation testing for all three products.
The manual Mutants workflow retains the three-product, four-shard campaign.
Don't require manual campaigns as ordinary pull-request status checks.
Confirm Mend-hosted Renovate has no open update pull requests and code and dependency scanning have no high or critical alerts before approving a release candidate.

## `release` environment

Restrict deployment branches to `main` only.
Configure at least one required reviewer who isn't the workflow initiator.
Turn off administrator and self-review bypass where the repository policy permits it.
Don't store `CRATES_IO_TOKEN` during preparation.
For the first publication only, add a narrowly scoped crates.io token immediately before approval.
Remove and revoke the token after verifying Trusted Publishing.

The manual [Release workflow](workflows/release.yml) names this environment and requires an exact candidate SHA, successful Release check run ID, version, authentication mode, and confirmation phrase.
Merging workflow files can't trigger publication.
Verify the environment settings before publication.
The workflow source doesn't establish their current provider state.

## Handoff record

Before the first irreversible action, record the verifying repository administrator, required reviewer identity, verification timestamp, candidate commit SHA, and successful Release check run ID outside the repository.
Also record the results of checking crates.io names, existing tags and releases, open dependency pull requests, and high or critical alerts.
