# ADR-0004: vertical writing is a direction, not a second implementation

- Status: accepted
- Date: 2026-08-05

## Context

Japanese is written both horizontally (横組み) and vertically (縦組み). The composition
rules are the same in both: the same character classes, the same prohibitions, the same
spacing. What differs is which axis the line advances along, and a small number of
constructs that genuinely change — tate-chu-yoko exists only vertically, and a handful of
characters take rotated or substituted forms.

The usual implementation treats vertical writing as a separate mode reached by a flag,
which duplicates the composition logic. The duplicate then drifts: a kinsoku fix lands in
the horizontal path and not the vertical one. This is why vertical support is so often
absent or half-correct — Typst had neither vertical writing nor ruby as of 0.14, and
retrofitting them into a horizontal-first model is the expensive part.

Deciding this after the horizontal implementation exists is too late. By then the
horizontal path has absorbed the assumption that "advance" means "x" in a hundred places.

## Decision

The core has no notion of x and y. A line advances along an *inline* axis and stacks along
a *block* axis, and composition is expressed entirely in those terms. Mapping inline and
block onto screen coordinates happens at the boundary, in the caller's renderer.

Horizontal and vertical composition therefore run the same code path. There is no vertical
module and no `if vertical` branch in the composition logic.

The constructs that genuinely differ — tate-chu-yoko, rotated forms, the vertical
alternates of brackets and the long vowel mark — are handled where they belong, as
per-character properties in `jlreq-class` and as inline constructs in `jlreq-inline`, not
as a fork of the line breaker.

This is decided now, before any composition code exists, precisely because it cannot be
retrofitted.

## Consequences

Vertical writing arrives at M5 as an addition to the class and inline layers rather than a
rewrite of the line layer, and every kinsoku fix applies to both directions by
construction.

The API talks about inline and block rather than width and height, which is unfamiliar at
first read. CSS writing modes use the same vocabulary for the same reason, so the concept
is not novel to anyone who has written a layout engine.

The same abstraction is what would make Mongolian or a right-to-left vertical script
expressible later. That is not a goal, but it is a sign the decomposition is along the
right seam.
