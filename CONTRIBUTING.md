# Contributing

## Setup

```sh
mise install        # toolchain and gate tooling
mise run hooks      # install the git hooks
just                # list the available commands
```

## The loop

```sh
just check          # fast deterministic gates
just ci             # everything CI runs, locally
```

`just ci` is also the pre-push hook. If it passes locally it passes in CI; if it fails,
fix the cause rather than narrowing the gate.

## Rules that are not negotiable

- **No `allow` and no `ignore`.** Every gate is strict on purpose. Make the code pass
  instead of suppressing the finding. If a lint is genuinely wrong for this codebase,
  change the shared configuration and say why in the commit message.
- **The core stays pure.** `jlreq-class`, `jlreq-spacing`, `jlreq-line`, `jlreq-inline`,
  and `jlreq` must not gain `std`, I/O, font, or floating-point dependencies. `just
  purity` enforces this; see [ADR 0001](docs/adr/0001-no-std-no-io-no-font-in-core.md) and
  [ADR 0005](docs/adr/0005-integer-layout-units.md).
- **Specification data is generated, not transcribed.** Class and spacing tables come from
  the published JLReq tables through a generator. A hand-edited table entry is a bug even
  when it is correct, because the next specification revision will not carry it forward.
- **Every rule gets a conformance case.** A rule without an entry in `jlreq-conform`
  addressed to the JLReq section it implements is incomplete.

## Code and comments are in English

The repository — including comments and documentation — is written in English so that the
spell checker works and so that adopters outside Japan can read it. Japanese terms of art
(kinsoku, mojikumi, oikomi) are used as loanwords with the kanji in parentheses on first
use, because they have no accurate English equivalents.

## Commits

Conventional Commits, validated by `committed` in the commit-msg hook:

```text
feat(class): add cl-1 through cl-5 determination
fix(line): stop hanging a closing bracket past the line end
docs(adr): record the integer-unit decision
```

## Where discussion belongs

Disagreements about what JLReq requires belong in `jlreq-conform` as a test case with the
section reference, not in an issue thread. Where JLReq permits alternatives, the answer is
a caller-visible option, not a default chosen in code review.
