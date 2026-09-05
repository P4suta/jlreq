<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# What must be true of any layout

[`docs/design/tracing.md`](tracing.md) covers the channel that says *why* a layout came out
as it did. `jlreq_core::verify` covers a different question: whether the layout is
self-consistent at all.

The two are complementary and neither replaces the other. A trace can be perfectly readable
about a composition that dropped a cluster; a sound layout can still be the wrong one. What
the checker adds is a floor — a set of statements that must hold for *every* layout,
whatever the input, so a fuzz target has something to assert instead of discarding its
result.

## Asking

```rust,ignore
let layout = jlreq_core::compose(&paragraph, &style)?;
let report = jlreq_core::verify::inspect(&layout, &paragraph);
assert!(report.is_sound(), "{report}");
```

The paragraph is required because most of these invariants are relational: a line's bytes
mean nothing without the source they index, and an attachment's construct ordinal means
nothing without the construct list it indexes.

It reports rather than panics. This workspace has two `debug_assert!` and no `assert!` in
shipped code, so an invariant is a typed value the caller decides what to do with — the same
shape `Paragraph` validation already has. A test asserts on the report; a fuzz target prints
it; a renderer could refuse to draw and say why.

## What it checks

| fault | the statement it holds |
| --- | --- |
| `extent-is-negative` | a line occupies a non-negative extent on both axes |
| `block-progression-reverses` | lines progress in one direction and never turn back |
| `coverage-starts-late` | the first line starts at the start of the source |
| `coverage-ends-early` | the last line reaches the end of the source |
| `lines-do-not-meet` | consecutive lines meet end to end, leaving no gap and no overlap |
| `line-range-is-inverted` | a line's own byte range runs forwards |
| `cluster-escapes-its-line` | a placement claims only bytes the line claims |
| `cluster-orientation-unexplained` | a placement in another writing mode says why, through its transform |
| `cluster-advance-is-negative` | a placement advances forwards |
| `placement-names-nothing` | a placement is attributed to a cluster or construct that exists |
| `attachment-names-no-construct` | an attachment names a construct that exists |
| `attachment-escapes-its-annotation` | an attachment claims only bytes its annotation stream holds |

Together the first six say the lines *partition the source* — the property that would break
first if composition ever dropped or duplicated input, and the one nothing else in the
workspace states.

Two of these are subtler than they look, and the golden corpus corrected both before they
could ship as false assertions:

- An `Attachment`'s range indexes the **annotation stream**, never the paragraph. A ruby
  attachment legitimately names bytes the base range does not hold, and a repeated
  emphasis mark has no stream at all, so it must claim an empty range.
- A placement may legitimately be set in a writing mode other than the paragraph's — that
  is precisely what tate-chu-yoko is. What must hold is that the local transform explains
  it.

## What it deliberately does not check

Anything that would need the composer's own intermediate state. "The ladder distributed
exactly what it needed to" is a statement about numbers that never reach `Layout`, and
re-deriving them here would make the checker a second composer whose agreement with the
first proves nothing. Those belong to the trace, which records the arithmetic as it happens,
and to `pipeline.rs`'s `oracle_chosen`, which is a deliberately naive second implementation
rather than a restatement of the first.

`verify` sits in its own layer in `xtask/src/direction.rs` that may not reach `pipeline`, so
that separation is mechanical rather than a matter of care.

## Where it runs

1. Its own unit tests, which build layouts the composer would never produce so each fault
   has a witness that states exactly one broken thing.
2. `crates/jlreq-core/tests/trace_goldens.rs`, over the whole golden corpus. That corpus was
   assembled to make the composer's reasoning branch, which makes it the widest set of
   composed layouts in the crate — every kinsoku level, both writing modes, hanging
   punctuation, ruby, warichu, furawake, tate-chu-yoko — so every invariant gets far more
   witnesses than its own tests can supply.
3. `fuzz/fuzz_targets/composition.rs`, which asserts soundness on every layout it composes.
   A fuzzer is the only thing here that tries inputs nobody thought to write a test for.

## Adding an invariant

Add a `Fault` variant. It will not compile until `Fault::kind` and `Fault::line` name it —
two wildcard-free tables, the same
[ADR 0012](../adr/0012-outcome-and-detail-compatibility.md) frozen-projection discipline
`jlreq_core::trace::Fact` follows. Then add it to `every_fault`, whose completeness test is
also wildcard-free.

Then run it against the golden corpus **before** adding an assertion anywhere else. An
invariant that is wrong about the model rather than about the composer looks exactly like a
bug until the corpus tells you otherwise, and it is much cheaper to learn that here than
from a fuzz crash.
