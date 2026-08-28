# Repository settings required for release

These controls live in GitHub and cannot be made reproducible by a workflow in this tree.
A repository administrator must verify them against the candidate and record the reviewer
and verification date in the release handoff. No crate credential belongs in these
settings during preparation.

## Actions and branch protection

- Require every third-party GitHub Action to be pinned to a full 40-character commit SHA.
  The checked-in workflows already satisfy this; keep the platform setting enabled so a
  later workflow cannot weaken it.
- Protect `main` and require the aggregate CI job plus CodeQL, dependency review, REUSE,
  API/semver, all four mutation shards for all three products, and the manual Release Check
  used by a candidate.
- Require the branch to be current before merge and prevent required checks from being
  bypassed by ordinary maintainers.
- Confirm Dependabot has no open update pull request and code/dependency scanning has no
  high or critical alert before approving a release candidate.

## `release` environment

- Restrict deployment branches to `main` only.
- Configure at least one required reviewer who is not the workflow initiator.
- Disable administrator/self-review bypass where the repository policy permits it.
- Do not store `CRATES_IO_TOKEN` during preparation.
- For the first publication only, add a narrowly scoped crates.io token immediately before
  approval, then remove and revoke it after Trusted Publishing is verified.

The manual [Release workflow](workflows/release.yml) names this environment and requires an
exact candidate SHA, successful Release Check run ID, version, authentication mode, and
confirmation phrase. Merging workflow files cannot trigger publication.

## Handoff record

Before the first irreversible action, record outside the repository:

- repository administrator who verified these controls;
- required reviewer identity;
- verification timestamp;
- candidate commit SHA and successful Release Check run ID; and
- the result of checking crates.io names, existing tags/releases, open dependency PRs, and
  high/critical alerts.
