<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# ADR-0033: one character size is two numbers, in the drawing contract too

- Status: accepted
- Date: 2026-09-07
- Completes [ADR 0007](0007-two-scalars-and-the-fixed-point-unit.md) and
  [ADR 0019](0019-one-fact-one-carrier.md); unblocks the deferral in
  [ADR 0027](0027-the-layout-is-the-editor-surface.md).

## Context

JLReq §3.3.3 names two ruby sizes. The principal one is half the base. The other is
三分ルビ, whose **block** extent is half the base em and whose **inline** extent is a
third — a reading that is condensed, not merely small.

[ADR 0007](0007-two-scalars-and-the-fixed-point-unit.md) took that seriously and made the
core's `Size` anisotropic for exactly this reason, offering the bridge once per axis so
that no call site can put an inline length on the block axis.
[ADR 0019](0019-one-fact-one-carrier.md) then made the declared size the *only* carrier of
the ruby em, deleting `RubySize` and `Question::RUBY_SIZE`, on the grounds that §3.3.3 does
not close its own set: for headings at twelve points or more it says only that the ruby "is
generally smaller than half the size of the base characters", with no ratio at all, and a
two-or-three-valued type cannot state that third case.

The facade never used any of it. `PreparedText::to_core` built `Size::square` twice — once
for the text's default em and once per cluster — and `annotation_options` fixed every
annotation at `font_size / 2`. Neither the caller nor the composer could say anything else,
so **a size JLReq explicitly names could not be requested through the public API**, and the
anisotropy ADR 0007 built and ADR 0019 relied on was reachable only from `jlreq-core`.

The obstacle was the far end. Even with an anisotropic `Size` reaching the composer, a
renderer had one number to draw with: `GlyphPlacement::font_size`. Set the face at half and
the outlines are half an em wide in a third of an em of advance; set it at a third and they
are a third of an em tall in half an em of line. Either way the drawn text and the cells
disagree, which is the class of defect `docs/design/geometry.md` and `jlreq::verify` exist
to make impossible. [ADR 0027](0027-the-layout-is-the-editor-surface.md) recorded that and
deferred the whole thing until the drawing contract had a design of its own to add a scale
channel to. [ADR 0029](0029-the-coordinate-system-is-a-contract.md) wrote that contract
down; this is the channel.

## Decision

**The drawing contract states one character size as two numbers, and the caller declares
the ruby's.**

`GlyphPlacement::font_size` keeps its meaning exactly: the size the face is *set* at, which
is the block-axis em. `GlyphPlacement::inline_size` is the em across the inline axis. They
are equal for every glyph the library has ever produced, and a renderer that ignores the
new one is correct for all of them; where they differ it sets the face at `font_size` and
scales the inline axis by `inline_size / font_size`.

`RubyScale` is the caller's statement of the size, held in units of 1/720 of the base em —
ADR 0007's unit, chosen so that a quantity at either named ruby scale is exact when restated
in base ems. `RubyScale::HALF` and `RubyScale::THIRD` are §3.3.3's two; `RubyScale::try_new`
is its open third case. It reaches the composer through
`LayoutOptions::with_ruby_scale`, and it moves the reading and nothing else: §3.3.9 fixes
the emphasis dot at half the base and makes it no one's parameter, and a reference mark and
a superscript are sized the same way.

The lowering shapes the reading at its **block** em, because that is the size a renderer
sets the face to, and then condenses: `PreparedText::condense_inline` scales each cluster's
advance, each glyph's inline advance and offset, and the cluster's `inline_size` by the
inline em over the block em. The axis it narrows is the text's — x in horizontal writing,
y in vertical — so the same declaration is right in both, which is the thing a screen-axis
implementation would get half right.

## Consequences

- Nothing moves at the default. `RubyScale::HALF` resolves both axes to the same em, the
  condensation is a no-op, and every existing test, golden and conformance case is
  byte-identical. The change is additive in the strict sense.
- `crates/jlreq/tests/construct_geometry.rs`'s
  `a_three_part_ruby_is_condensed_across_the_inline_axis` pins the numbers in both writing
  modes: the face stays at 512 of the paragraph's 1024, the inline em becomes 341 — which
  is 1024 × 240/720 truncated — and the cell narrows with the advance rather than
  overhanging what the composer reserved. `verify::inspect` reports the result sound, which
  is the statement that the drawn text and the cells still agree.
- The cell needs no correction of its own: a cell's inline extent is the composer's
  advance, and the advance was condensed with the em. Only the outline does.
- `Cluster::with_size` now receives `Size::new(inline, block)` for every cluster rather
  than `Size::square`, so the facade stops discarding an axis the core has carried since
  ADR 0007.
- Still the caller's, and deliberately: the per-construct override ADR 0019's third
  precedence rule allows is not built. `with_ruby_scale` is a policy, and a document that
  mixes 二分 and 三分 readings cannot yet say so. That is the next thing here, and it is a
  smaller change than this one because the channel now exists.
