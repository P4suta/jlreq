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

These are enforced mechanically by `just purity`, `just no-std`, and `just wasm`, not by
convention. A change that violates one fails CI.

1. **The core has no std, no I/O, and no font access** ([ADR 0001](docs/adr/0001-no-std-no-io-no-font-in-core.md)).
   `jlreq-class`, `jlreq-spacing`, `jlreq-line`, `jlreq-inline`, and `jlreq` are `no_std`.
   Reading a file, opening a font, or allocating an OS resource is out of scope by
   construction rather than by discipline.

2. **Metrics come from the caller** ([ADR 0002](docs/adr/0002-caller-supplied-metrics.md)).
   The library never asks how wide a character is; it is told. This is what keeps
   invariant 1 possible and makes every test a fixed-input/fixed-output comparison.

3. **Composition is expressed above ICU4X and HarfRust** ([ADR 0003](docs/adr/0003-layer-above-icu4x.md)).
   Break opportunity discovery and glyph shaping are not reimplemented.

4. **Vertical writing is a direction, not a mode** ([ADR 0004](docs/adr/0004-writing-mode-abstraction.md)).
   Horizontal and vertical composition run the same code path. There is no second
   implementation to keep in sync.

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
| `jlreq-class` | — | no |
| `jlreq-spacing` | `jlreq-class` | no |
| `jlreq-line` | `jlreq-class`, `jlreq-spacing` | yes (`alloc`) |
| `jlreq-inline` | `jlreq-class`, `jlreq-spacing` | yes (`alloc`) |
| `jlreq` | all of the above | yes (`alloc`) |
| `jlreq-conform` | `jlreq` | yes |

`jlreq-class` and `jlreq-spacing` are pure lookups over static tables and allocate
nothing, so they are usable from an interrupt handler or a shader-adjacent context. The
dependency graph is acyclic and shallow on purpose: a consumer that only needs character
classification never pulls in line composition.

## What lives where

- **Specification data** (class tables, spacing tables) is generated from the published
  JLReq tables rather than transcribed, so a specification revision is a regeneration.
- **Rules** (kinsoku, adjustment, hanging) are expressed as data plus a small evaluator,
  not as branching code, so the conformance suite can address individual rules.
- **Policy** (which of several permitted behaviors to choose) is a caller-visible option,
  never a hardcoded default buried in the evaluator. JLReq permits alternatives in several
  places, and publishers disagree about them.
