<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading a decision trace

`jlreq_core::trace` answers the question the result cannot: not *what* the composer
produced, but *why*. [ADR 0028](../adr/0028-the-trace-is-not-a-diagnostic.md) records why
that is a separate channel from `Layout` and `Diagnostic` rather than an extension of
either.

## Recording one

```rust,no_run
use jlreq_core::trace::{Categories, Trace};

# fn example(paragraph: &jlreq_core::Paragraph, style: &jlreq_core::Style)
#     -> Result<(), jlreq_core::ComposeError> {
let mut trace = Trace::new();
let layout = jlreq_core::compose_traced(paragraph, style, &mut trace)?;

// One decision per line, already formatted.
println!("{trace}");

// Or walk it: every event names its kind, where it belongs, and the rule it rests on.
for event in trace.events() {
    let _ = (event.kind(), event.site().bytes(), event.jlreq());
}
# let _ = layout;
# Ok(())
# }
```

`compose` records nothing and allocates nothing for the sink, so the untraced path costs a
load, a mask, and a branch that is never taken. Both entry points run one body.

## Choosing what to record

`Categories` is a set. `Categories::DEFAULT` records every family whose event count stays
proportional to lines times sites. Two families are outside it because they are quadratic
or linear in the input rather than in the output:

- `SEARCH_CANDIDATES` — every break pair the search weighed. A ten-thousand-cluster
  paragraph charges hundreds of thousands of these.
- `PLACE_CLUSTERS` — every placed cluster.

Turn one on deliberately, on a paragraph you have already narrowed down:

```rust,no_run
# use jlreq_core::trace::{Categories, Trace};
let mut trace = Trace::with_categories(Categories::DEFAULT.with(Categories::SEARCH_CANDIDATES));
# let _ = trace.categories();
```

A trace stops at `Trace::DEFAULT_MAX_EVENTS` and reports `is_truncated`. Truncation is not
a `ComposeError`: a debugging aid must never turn a composable paragraph into a refusal.

## What is recorded

| family | kinds | answers |
| --- | --- | --- |
| `PREPARE` | `prepare.paragraph` | how big the problem was, and whether the indexed fast path applied |
| `SEARCH` | `search.chosen`, `search.refused` | which breaks won, what each line cost, how far a refused search got |
| `SEARCH_CANDIDATES` | `search.candidate`, `search.bound-stop` | every pair weighed, its cost broken out, and where the search stopped extending |
| `KINSOKU` | `search.refused-candidate` | which boundaries kinsoku (禁則) refused outright |
| `SPACING` | `space.boundary` | the class pair, the cell terms, and the amount used |
| `REDUCE` | `reduce.site`, `reduce.stage` | what each boundary could give up, and what each rung took |
| `EXPAND` | `expand.site`, `expand.stage`, `expand.residual` | the ceilings, the rungs, and what fell past all of them |
| `HANGING` | `hang.line-end` | what was hung past the measure rather than absorbed |
| `STRUCTURE` | `warichu.block`, `furawake.block`, `tcy.group` | how a stacked structure was cut, dealt, or set upright |
| `PLACE` | `line.fit`, `line.finished` | what the line was asked to absorb, and what it came out as |
| `PLACE_CLUSTERS` | `place.cluster` | every placement, with its local transform |

## The line format

```text
jlreq.trace/1 events=10 categories=0x07fb truncated=0
0000 prepare.paragraph        P   c0..9 b0..27 clusters=9 candidates=10 constructs=0 fast=1 extent=2500 mode=horizontal-tb align=justify
0001 search.refused-candidate P   c1..1 b3..3 candidate=1 3.1.9
0005 search.chosen            L00 c0..2 b0..6 line=0 start=0 end=2 edge=0 3.1.1
```

| column | meaning |
| --- | --- |
| `0000` | the event ordinal, zero-padded so a diff aligns |
| `prepare.paragraph` | the kind, padded to a fixed width |
| `P` / `L00` | paragraph scope, or the line ordinal being set |
| `c0..9` | shaped-text cluster ordinals; a single ordinal prints as `c4` |
| `b0..27` | source UTF-8 byte range |
| `clusters=9 …` | the fields this kind fixes, always in the same order |
| `3.1.9` | the rule the decision rests on, where one states it |

Booleans print as `0` and `1` so columns line up. Nothing is rendered through `Debug`,
because `Debug` is not a stable format and a golden written against it would be pinned to
the compiler rather than to this crate.

Rule addresses follow [ADR 0013](../adr/0013-rules-are-addressed-by-specification-address.md):
a section is `3.8.3`, an appendix note is `C.2#5`, and a table cell is `B.1@cl-05,cl-05`.

## What the search says, and what it deliberately does not

Every weighed candidate carries **both** its natural width and its width after the
available reduction is spent:

```text
0031 search.candidate         P   c12..40 b36..120 start=12 end=40 natural=4320 reduced=4180 avail=4000 delta=-180 badness=32410000 disc=0 warichu=0 formula=0 widow=0 edge=32410000 total=58010000 last=0 accepted=0 3.8.1
0032 search.bound-stop        P   c12..40 b36..120 start=12 end=40 minimum=4160 avail=4000 best=25600 3.8.1
```

The reduction capacity the search assumed for that line is `natural - reduced` — a
subtraction the reader performs. That is why the ladder families stay silent while the
search runs: preparation and search reach the same helpers that placement does, but they
are measuring rather than setting, and preparation in particular calls them with a boundary
of zero for every cluster because it is summing a paragraph. An unphased event from there
would name a boundary that is not one.

Kinsoku is the exception and does speak during preparation, because a kinsoku refusal names
the boundary it actually refused.

## The goldens

`crates/jlreq-core/tests/goldens/` holds a rendered trace per scenario, byte for byte,
checked by `crates/jlreq-core/tests/trace_goldens.rs` on every `just test`.

This is the project's push-triggered drift oracle. The three-implementation census
([ADR 0024](../adr/0024-independent-reference-engines.md)) is stronger but needs OCaml and
Racket toolchains `mise` does not manage, and runs only from a manually dispatched
workflow. The goldens run everywhere, on all three operating systems, and compare
*reasoning* rather than answers — so a change that reorders the ladder or charges a
different surcharge moves a golden even where the final geometry agrees. Geometry agreeing
by coincidence is the case that would otherwise ship unnoticed.

To adopt an intended change:

```sh
JLREQ_BLESS=1 cargo test -p jlreq-core --test trace_goldens
```

Read the diff before committing it, and say in the commit message why the reasoning moved.
A golden that changes without a stated reason is the finding, not the noise.

Two further tests keep the corpus honest: one asserts it still reaches every family it was
assembled for, so a scenario cannot quietly stop exercising anything while its golden keeps
passing, and one asserts recording does not change the layout of any recorded scenario.

## Adding a fact

`Fact` is `#[non_exhaustive]` and pairs with three wildcard-free tables — `kind`,
`category`, and `jlreq` — plus the renderer. None has a `_ =>` arm, so a new variant does
not compile until all four name it, which is
[ADR 0012](../adr/0012-outcome-and-detail-compatibility.md)'s frozen-projection requirement
met by the compiler rather than by review.

Two rules a new kind must follow:

1. Its name must not begin with `input.`, `style.`, `compose.`, `layout.`, `font.`,
   `document.`, or `limit.`. Those are the product error-code namespaces `xtask repository`
   holds against [docs/error-codes.md](../error-codes.md) in both directions, and a trace
   name is not a compatibility key. A unit test asserts this.
2. It must record the derivation, never restate a result. There is no `Fact::Overfull`
   beside the `layout.overfull` diagnostic; there is the geometry that produced it.
