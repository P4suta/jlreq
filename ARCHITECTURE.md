# Architecture

This document states the invariants. The reasoning behind each one lives in
[`docs/adr/`](docs/adr/); this file is the summary a reader needs before touching code.

## Position in the stack

```text
   application / Typst / Parley / PDF writer / game engine
                          ▲
                      kumihan                ← line composition only
                          ▲
   ICU4X (UAX #14 break opportunities) / HarfRust (shaping)
```

kumihan replaces nothing. It consumes break opportunities and shaped advances, applies the
Japanese rules that neither layer expresses, and hands positions back. An adopter keeps
its existing text stack.

## Invariants

These are enforced mechanically by `just purity`, `just no-std`, `just wasm`, and
`just direction`, not by convention. A change that violates one fails CI.

1. **The core has no std, no I/O, and no font access** ([ADR 0001](docs/adr/0001-no-std-no-io-no-font-in-core.md)).
   `jlreq-unit`, `jlreq-spec`, `jlreq-class`, `jlreq-spacing`, `jlreq-line`, `jlreq-inline`,
   and `jlreq` are `no_std`.
   Reading a file, opening a font, or allocating an OS resource is out of scope by
   construction rather than by discipline.

2. **Metrics come from the caller** ([ADR 0002](docs/adr/0002-caller-supplied-metrics.md)).
   The library never asks how wide a character is; it is told. This is what keeps
   invariant 1 possible and makes every test a fixed-input/fixed-output comparison.

3. **Composition is expressed above ICU4X and HarfRust** ([ADR 0003](docs/adr/0003-layer-above-icu4x.md)).
   Break opportunity discovery and glyph shaping are not reimplemented.

4. **Vertical writing is a direction, not a mode** ([ADR 0004](docs/adr/0004-writing-mode-abstraction.md),
   [ADR 0011](docs/adr/0011-typed-axes-and-direction-as-a-datum.md)).
   Not that nothing reads the direction: JLReq conditions exactly three rules on it —
   §3.1.3, §3.2.5, and §3.3.5 — and the invariant is that those three are the whole of it.
   The generated rule inventory marks them, `docs/direction-sites.toml` names the item that
   reads each, and `just direction` fails unless the two sets are equal, so a fourth branch
   is a change to generated data plus a code-owner review rather than an incidental `if`.
   The inventory is generated whole while the reading half is written milestone by
   milestone, so a marked rule whose reader has not been written carries a `[[pending]]`
   entry in that same file naming the crate whose first item closes it. The gate reports
   each one by name as a rule the equality did not run over, and the entry is itself a
   violation once anything reads the rule or once that crate declares an item — a statement
   about a subject that does not exist yet, never an exemption.
   Everything else the specification states twice is exact axis mapping, and the inline and
   block axes are distinct types with no conversion in either direction. A branch that reads
   the direction indirectly is invisible to that gate, so the conformance suite composes
   every case that is not direction-specific both ways and requires bit-identical inline
   results.

5. **Layout arithmetic is integer** ([ADR 0005](docs/adr/0005-integer-layout-units.md)).
   No floating point anywhere in the core. Output is bit-identical on every target, golden
   tests compare exactly rather than within a tolerance, and the core runs on targets
   without a floating-point unit.

6. **The conformance suite is a deliverable** ([ADR 0006](docs/adr/0006-conformance-suite-as-artifact.md)).
   `jlreq-conform` maps one-to-one onto JLReq sections and is published for others to run,
   including against implementations that are not this one.

## Crate boundaries

| Crate | Depends on | Allocates |
| --- | --- | --- |
| `jlreq-unit` | — | no |
| `jlreq-spec` | `jlreq-unit` | no |
| `jlreq-class` | `jlreq-unit`, `jlreq-spec` | no |
| `jlreq-spacing` | `jlreq-unit`, `jlreq-spec`, `jlreq-class` | no |
| `jlreq-line` | `jlreq-unit`, `jlreq-spec`, `jlreq-class`, `jlreq-spacing` | yes (`alloc`) |
| `jlreq-inline` | `jlreq-unit`, `jlreq-spec`, `jlreq-class` | yes (`alloc`) |
| `jlreq` | all of the above | yes (`alloc`) |
| `jlreq-conform` | `jlreq`, `jlreq-spec` | yes |

`jlreq-unit` holds quantities, axes, and the item vocabulary; `jlreq-spec` holds rule
addresses, provenance, and the policy space; `jlreq-class` and `jlreq-spacing` are lookups
over static tables. None of the four allocates, so all four are usable from an interrupt
handler or a shader-adjacent context. The dependency graph is acyclic and shallow on
purpose: a consumer that only needs character classification never pulls in line
composition, and one that needs only the quantities pulls in nothing at all.

`jlreq-spec` reaches `jlreq-unit` for one function
([ADR 0020](docs/adr/0020-the-seam-carries-no-rule-address.md)). `Policy::remainder` is the
single derivation of a remainder rule from a policy, which is what keeps the policy the one
carrier of a choice that `distribute` takes as a parameter
([ADR 0019](docs/adr/0019-one-fact-one-carrier.md)); the edge runs this way and not the
other so that `jlreq-unit` still depends on nothing, and the seam types consequently carry
no rule address. The table states the permitted adjacency, which is what `just purity`
checks; a crate declares an edge in the commit whose code needs it.

The two absent edges are load-bearing ([ADR 0015](docs/adr/0015-the-crate-graph-and-the-inline-line-seam.md)).
`jlreq-inline` does not reach `jlreq-line`, because §3.4.3 lets a warichu (割注) straddle
two lines, so its interior's available measure is unknown until the outer break is chosen
and every break selection in the workspace therefore happens in `jlreq-line`; and
`jlreq-line` does not reach `jlreq-inline`, because the line layer resolves the ruby
overhang allowance and the construct layer places against an allowance it is told.
Everything crossing that seam lives in `jlreq-unit`, so neither crate names a type the
other owns, and `just purity` checks this exact adjacency rather than mere core membership.

## What lives where

- **Specification data** is generated where W3C publishes it machine-readably and attested
  where it does not ([ADR 0009](docs/adr/0009-generated-data-and-attested-transcription.md)).
  Appendix A's keys, the legends, the notes, the ladders, and the rule inventory are
  generated from an HTML snapshot, so a specification revision is a regeneration and a hand
  edit is a bug. The spacing matrices exist only as PDF, with the priority ordinals encoded
  as cell background color and the color key published as an image, so their roughly 5400
  cells are transcribed: entered twice, from the English and the Japanese rendering, every
  cell carrying its source, and checked against cross-table invariants derived from prose
  that is machine-readable.
- **Rules** — kinsoku (禁則), adjustment, hanging — are expressed as data plus a small
  evaluator, not as branching code, so the conformance suite can address individual rules.
- **Policy** (which of several permitted behaviors to choose) is a caller-visible option,
  never a hardcoded default buried in the evaluator. JLReq permits alternatives in several
  places, and publishers disagree about them.
