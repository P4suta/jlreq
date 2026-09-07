<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Finding: ADR 0017's normalization is a decision, not yet a behavior

- Applies to: `jlreq_core::Frame`, `jlreq_core::Line`, `spec::narrow_by_frame`,
  `spec::table_one_space_components`, `xtask/src/conform.rs`'s `check_trims`
- Standing: `Adjudicated` by [ADR 0017](../adr/0017-normalized-line-geometry.md); the
  decision stands and is unimplemented
- JLReq: §3.1.2, Table 1

## What ADR 0017 decided

> A caller-declared frame that already contains a conditional space is normalized by
> subtracting that amount, and the subtraction is reported: `Line::trims` names the item,
> the amount, the side it came off, and the rule that states it.

The ADR's own Context says why this matters at scale rather than at the margins: §3.1.2
covers commas (cl-07), full stops (cl-06), opening brackets (cl-01), closing brackets
(cl-02) and middle dots (cl-05) — the five commonest punctuation classes in Japanese — so a
disagreement about them is "a systematic half-em error across the whole corpus rather than
an edge case."

## What 0.1.0 actually does

Measured, and now pinned by `crates/jlreq-core/tests/frame_normalization.rs`: a comma and an
ideograph, both supplied at one em, composed under `Style::jlreq_2020()`.

| declared frame | line extent | `、` advance | `日` advance |
| --- | --- | --- | --- |
| `Frame::FullEm` | 2500 | 1500 | 1000 |
| `Frame::HalfEm` | 2500 | 1500 | 1000 |
| `Frame::Proportional` | 2500 | 1500 | 1000 |

The declared frame changes nothing that reaches the caller. Under ADR 0017, `FullEm` would
report 500 for the comma — the supplied em less the half-em §3.1.2 says is already inside it
— and only `HalfEm` would report 1500.

Four facts explain the table, and each is checkable:

1. **`Frame` reaches only classification.** `spec::narrow_by_frame` narrows the candidate
   classes for cl-01, cl-02, cl-05, cl-06 and cl-07 by whether the advance states the
   character's width. Nothing downstream of it consults the frame again.
2. **The space is only ever added.** `spec::table_one_space_components` returns amounts that
   the boundary computation adds. There is no subtracting path anywhere in the core.
3. **No validation detects the contradiction.** ADR 0019 refers to a
   `FrameContradictsAdvance` error as something the design no longer needs; the identifier
   appears nowhere in the workspace but in that sentence, so a caller declaring `FullEm`
   with a half-em advance is neither normalized nor refused.
4. **The report has no carrier.** `jlreq_core::Line` states seven things and `trims` is not
   among them. `docs/design/api-spine.md` does not name it either.

The conformance side is written and idle in the same way: `xtask/src/conform.rs`'s
`check_trims` validates that each `trims` entry cites §3.1.2 or a Table 1 cell such as
`B.1@cl-05,cl-05` — and neither `suite.ndjson` nor `protocol.schema.json` contains the word,
so it runs against fixtures only. Both halves are waiting for the same change.

## Why it is being left alone

Implementing ADR 0017 faithfully is not adding a report. It is changing the composed
geometry of the five commonest punctuation classes across every input that declares an
ideographic frame.

The oracle that would catch a mistake in a change of that size is the three-implementation
census — 122,199 requests against independent OCaml and Racket engines, currently agreeing
with zero differences ([ADR 0024](../adr/0024-independent-reference-engines.md)). It cannot
be run from this repository: those engines need toolchains `mise` does not manage, and
`just census-all` runs only from a manually dispatched `Release check`. Making a systematic
half-em change under an oracle that cannot be consulted is the one thing the project's own
constraints forbid outright.

There is a second reason, independent of tooling: "does a declared full-em frame mean the
space is added or already present" is a substantive typesetting decision about what a
caller's `Frame` *asserts*, and the ADR settled it in one direction while the implementation
went the other. Reconciling them deserves its own deliberation, not a change made in passing
to satisfy a gate.

## What would resume it

1. OCaml and Racket toolchains available, so `just census-all` can be run before and after.
2. The trimming behavior implemented in `spec`, with `Frame` consulted where the boundary
   amount is computed rather than only where classes are narrowed.
3. `Line::trims` added, carrying item, amount, side and rule address, and registered in
   `docs/public-api.toml` and `docs/design/api-spine.md`.
4. The protocol extended and regenerated — `suite.ndjson` and `protocol.schema.json` are
   generated artifacts pinned by digest in `data/manifest.toml`, so `check_trims` starts
   running for real the moment a case carries the field, and the reference engines must
   follow before the census can pass.
5. `crates/jlreq-core/tests/frame_normalization.rs` updated in the same commit — it exists
   to fail when this happens.

## Detection in the meantime

The trace's `space.boundary` events name the table, the class pair, the cell terms and the
amount actually used:

```text
0021 space.boundary  L00 c2 b6..9 before=19 after=15 bsize=1024 asize=1024 bsolid=0 asolid=0 terms=0,0 applied=0 B.1@cl-19,cl-15
```

`crates/jlreq-core/tests/goldens/punctuation-mojikumi.txt` records those for the five
classes byte for byte, so any movement in this area shows up as a golden diff rather than as
a silent corpus-wide shift. See [`design/tracing.md`](../design/tracing.md).
