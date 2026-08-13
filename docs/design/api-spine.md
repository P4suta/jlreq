# The 1.0 API spine

This is the human-readable contract for the only public Rust library, `kumihan`. The exact
directly exported names and the 22 Style mappings are machine-readable in
[`docs/api-1.0.toml`](../api-1.0.toml) and checked in both directions by `xtask api`.

This document describes the 1.0 target while publication remains disabled. The old
multi-crate API is not carried forward and has no compatibility facade.

## Principles

- All positions and ranges in input text are UTF-8 byte coordinates.
- All public geometry is a bounded `i32` in the caller's unit.
- Inputs are already shaped. Font I/O, shaping, UAX #14, bidi, and rendering are out of
  scope.
- `ParagraphBuilder::build` is the validation boundary. Composition of a validated
  paragraph does not fail.
- Classification, spacing records, lowering seams, feasibility, ladders, badness, and rule
  IDs are private.
- Public result types are read-only views with private fields.
- Every public type is `#[non_exhaustive]`; new detail may be added without changing an
  existing outcome.

## Normal path

```rust
let text = ShapedText::new(source, Size::square(1_000)?, Frame::FullEm, clusters)?;
let paragraph = Paragraph::builder(text, 20_000)
    .breaks(break_offsets.map(Break::allowed))
    .build()?;
let layout = kumihan::compose(&paragraph, &Style::book_2020());

for line in layout.lines() {
    for placement in line.clusters() {
        draw(placement);
    }
}
```

`Composer::compose` is the same operation with reusable search scratch space. The root
function is for one-off composition.

## Model

`Size` carries positive inline and block em lengths. `Frame` is an explicit `FullEm`,
`HalfEm`, or `Proportional` metrics interpretation. `WritingMode` is `HorizontalTb` or
`VerticalRl`.

`Cluster::new(range, advance)` requires only an original-source byte range and shaped inline
advance. Builder methods may override size, frame, and `ClusterRole` for an occurrence.
`ShapedText::new(source, size, frame, clusters)` owns and validates the complete stream.

Normalization may join an Appendix A two-code-point key across multiple shaped clusters,
but it preserves attribution to those clusters. A non-proportional cluster may not hide
multiple keys; proportional Latin ligatures remain valid.

## Paragraph

`Paragraph::builder(text, line_extent)` accepts:

- ordinary, mandatory, and discretionary `Break` values;
- first-line indent and logical `Alignment`;
- `Widow` control;
- `TabStop` values with logical `TabAlignment`;
- `WritingMode`; and
- a sequence of opaque `Construct` values.

The builder rejects invalid UTF-8/cluster boundaries, uncovered or overlapping clusters,
duplicate breaks and tab positions, crossing construct ranges, illegal breaks inside an
indivisible construct, invalid sizes, and invalid construct-specific counts.

The paragraph end is an implicit mandatory break. The only search exposed by composition is
whole-paragraph optimization.

## Inline structures

`Construct` is opaque and has nine named constructors:

| Constructor | Structure |
| --- | --- |
| `ruby` | mono, group, or jukugo ruby |
| `tate_chu_yoko` | upright horizontal span in vertical writing |
| `emphasis_dots` | repeated emphasis marks |
| `warichu` | inline cutting note |
| `furawake` | distribution into an explicit number of columns |
| `jidori` | fit into an explicit number of full-em cells |
| `reference_mark` | shaped reference mark attached to a base |
| `script` | shaped subscript/superscript complex |
| `formula` | indivisible shaped formula span |

Ruby alone has public supporting types: `Ruby`, `RubyKind`, and `RubyRun`. The annotation is
another `ShapedText`; runs map base ranges to annotation ranges. Internal runs,
contributions, seams, and placement strategies are not public.

## Style

`Style` is complete and immutable. `StyleBuilder` has one typed setter for each choice and
rejects contradictions at `build()`. Profiles are:

- `jlreq_2020` and the permanently equivalent `default`;
- `book_2020`;
- `magazine_2020`;
- `newspaper_2020`; and
- `jis_reading_2020` (the alternatives JLReq records, not complete JIS X 4051 conformance).

The `kumihan::style` namespace contains the 22 dedicated choice enums. Their complete names
and specification paths live in `docs/api-1.0.toml`; generic `Question`, `Choice`, and
string-setting types are intentionally absent.

## Results

`Layout::lines()` and `Layout::diagnostics()` return borrowed slices. A `Line` provides its
source range, logical origins, occupied inline extent, block demand, cluster placements,
and attachments.

Each `ClusterPlacement` provides:

- `PlacementOrigin` (source cluster or construct ordinal);
- original byte range;
- logical inline and block coordinates;
- placed advance, local size, and frame; and
- local writing mode and `CoordinateTransform`.

This is enough for a renderer to draw proportional text in vertical writing and
tate-chu-yoko without reshaping. `Attachment` provides the equivalent geometry and
attribution for ruby, emphasis marks, reference marks, and script annotations.

`Diagnostic` is opaque. Only its stable code, `Severity`, optional input range, and JLReq
reference string are contractual. It exposes no internal rule sequence.

## Errors and compatibility

`InputError` means no valid paragraph could be built; its stable code and optional range are
for programs, while its message may improve. `StyleError` similarly exposes a stable
conflict code.

`Style::default()` never changes meaning. A new specification revision adds a dated profile
and a new specification identifier. The process protocol is versioned separately.

The candidate release has no compatibility layer for the former `jlreq-*` API. Those
crates remain unpublished regression assets only until differential migration is complete.
