<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Using jlreq

`jlreq` lays out Japanese and mixed-script UTF-8 text from in-memory fonts.
Supply text and font bytes with a font size and line extent.
The result contains glyphs in visual order with coordinates and source ranges.
It also supplies transforms, bidi levels, and the font resources needed to draw the glyphs.

The 0.1.0 workspace has three products.
The `jlreq` library combines shaping, fallback, and bidi with line breaking and composition.
Engines with shaped clusters can use `jlreq-core` for dependency-free `no_std + alloc` composition.
The `jlreq-conformance` runner validates protocol-v1 behavior with the black-box suite.

Version 0.1.0 remains unreleased.
The release workflow prepares and verifies three crate archives before publication.
It also prepares six target-specific conformance binary archives and their checksums.
The workflow also prepares software bills of materials and attestations for the artifacts.
Preparation doesn't publish crates, create the `v0.1.0` tag, or create a GitHub Release.

## Work from source

Until publication, work from the source checkout:

```sh
git clone https://github.com/P4suta/jlreq.git
cd jlreq
mise install
```

The `jlreq` library requires Rust 1.88.
Both `jlreq-core` and `jlreq-conformance` build on Rust 1.85.
The crates don't include fonts.
Layout takes font bytes supplied by the caller.
The runnable examples read a font path from their first argument.
Use a TrueType or OpenType font such as [Noto Sans JP](https://fonts.google.com/noto/specimen/Noto+Sans+JP).
Font files commonly use the `.ttf` or `.otf` extension.
The optional `system-fonts` feature enables OS font discovery.
The [Japanese guide](guide.ja.md) describes usage in Japanese.

## Quick start

The following example is identical to the `jlreq` crate's compiled doctest.
The repository's example gate compares both copies and runs the guide's copy with a fixture font:

<!-- jlreq-example: quickstart -->
```rust
use jlreq::{FontLibrary, LayoutOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("pass a font file, e.g. NotoSansJP-Regular.otf")?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;

    let options = LayoutOptions::try_new(240.0, 16.0)?;
    let layout = jlreq::layout("日本語組版 — draw-ready glyphs", &fonts, options)?;

    for glyph in layout.glyphs() {
        if let Some(font) = layout.font(glyph.font_id()) {
            // Everything a renderer needs to draw this glyph:
            let _draw = (
                font.bytes(),
                font.face_index(),
                glyph.glyph_id(),
                glyph.draw_origin(),
                glyph.font_size_26_6(),
                glyph.variations(),
                font.synthesis(),
                glyph.transform(),
            );
        }
    }
    Ok(())
}
```

`TextLayout` owns every `FontResource` referenced by its glyphs.
IDs can be sparse, so renderers resolve them with `TextLayout::font` instead of indexing a slice.
Draw `glyph_id` at `draw_origin` using the returned size and effective variations.
Apply the font's synthetic emboldening/skew followed by `GlyphTransform`.
For decorations, `FontResource::metrics` supplies em-relative ascent and descent.
It also supplies x-height, cap height, and underline geometry.
The caller handles rasterization, GPU upload, PDF encoding, and drawing.

`GlyphPlacement::cell_bounds` gives physical layout-cell boundaries.
The same meaning applies to `TextLine::bounds` and `TextLayout::bounds`.
Those boundaries include whitespace and annotation cells.
For clipping and decorations, a rasterizer derives ink bounds from the selected outline and variations.

## Coordinate system

Physical coordinates follow the screen: **+x is right and +y is down**.
Each layout has an origin at the start of its first line.
The inline axis runs along a line, and the block axis runs from line to line.
In `VerticalRl`, only the block axis runs backwards, so a later column has a smaller `x`.

`GlyphPlacement::origin` is its cell's **inline-start, block-end** corner.
The `draw_origin` adds the shaper's offset to that point.
Neither position is the baseline.
An outline placed at the cell's block-end sits one descent too far along the block axis.
`FontMetrics::descent` supplies the negative, em-relative correction:

<!-- jlreq-example: baseline -->
```rust
use jlreq::{FontLibrary, FontMetrics, FontResource, LayoutOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("pass a font file, e.g. NotoSansJP-Regular.otf")?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;
    let layout = jlreq::layout("日本語組版", &fonts, LayoutOptions::try_new(240.0, 16.0)?)?;

    for glyph in layout.glyphs() {
        let cell = glyph.cell_bounds();
        let descent = layout
            .font(glyph.font_id())
            .and_then(FontResource::metrics)
            .map_or(0.0, FontMetrics::descent);
        // Where a rasterizer puts the outline's own origin.
        let _baseline = (
            cell.x(),
            cell.y() + cell.height() + descent * glyph.font_size(),
        );
    }
    Ok(())
}
```

The [coordinate contract](design/geometry.md) defines layout geometry.
It explains why a cell's advance differs from the distance to the next cell.
The `jlreq::verify::inspect` function checks layouts and returns a typed report without panicking.
Tests, fuzz targets, and editors can inspect the report.
The [`render_svg`](../crates/jlreq/examples/render_svg.rs) example draws layout cells and places text on the derived baseline.
The drawing makes the geometry visible.

Use `LayoutEngine` for batches.
The engine reuses parsed fonts, shaping data, Unicode services, and core-composer scratch space.
It remains reusable after an error.
Its results are bit-identical to the one-shot `jlreq::layout` call.

## Composition policy

A typed `Style` value represents composition policy.
The default profile is `Style::jlreq_2020()`.
Other predefined profiles include `book_2020()` and `magazine_2020()`.
The `newspaper_2020()` and `jis_reading_2020()` profiles complete the set.
The `StyleBuilder` type overrides individual choices recorded in the [public API inventory](public-api.toml).
Choices include kinsoku strictness, hanging punctuation, and ruby overhang.
The `ExpansionOrder` policy selects the order of space expansion operations.
Apply a policy with `LayoutOptions::with_style`.
For individual paragraphs, use `ParagraphStyle::with_style`.

## Fonts, fallback, vertical text, and bidi

The `FontLibrary::register_font` method derives the family name from the font's `name` table.
A span's family request then matches without extra registration metadata.
Use `FontLibrary::register_face` to record explicit family and style metadata for a font face.
The method accepts faces from TrueType or OpenType files, including font collections.
Those files commonly use the [extensions](https://learn.microsoft.com/en-us/typography/opentype/spec/otff#filenames) `.ttf`, `.otf`, and `.ttc`.
The primary face supplies `.notdef`, and `set_fallback_order` sets an explicit priority.
The first font covering an entire extended grapheme, including a variation sequence, wins.
If no font covers the grapheme, layout retains the source range and emits the primary `.notdef`.
It reports `font.missing-glyph` for the missing coverage.
A request for an undeclared span family reports `font.unknown-family`.

```rust no_run
use jlreq::{BaseDirection, FontLibrary, LayoutOptions, WritingMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let japanese = std::env::args()
        .nth(1)
        .ok_or("pass a Japanese font file first")?;
    let latin = std::env::args().nth(2).ok_or("pass a Latin font file second")?;

    let mut fonts = FontLibrary::new();
    let jp = fonts.register_font(std::fs::read(japanese)?)?;
    let latin = fonts.register_font(std::fs::read(latin)?)?;
    fonts.set_primary(jp)?;
    fonts.set_fallback_order([jp, latin])?;

    let options = LayoutOptions::try_new(320.0, 16.0)?
        .with_writing_mode(WritingMode::VerticalRl)
        .with_base_direction(BaseDirection::Auto);
    let layout = jlreq::layout("日本語 Latin \u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629}", &fonts, options)?;
    assert!(layout.glyphs().all(|glyph| !glyph.source_range().is_empty()));
    Ok(())
}
```

Horizontal and vertical results use physical coordinates.
Latin rotation, upright CJK, and tate-chu-yoko use those coordinates.
Per-line visual reordering follows [Unicode Standard Annex #9](https://www.unicode.org/reports/tr9/tr9-51.html).
Hit testing also uses physical coordinates.
The caller needs no logical-to-physical conversion for caret and selection rectangles.
The layout supports visual and line-to-line caret motion with grapheme boundaries.
It also provides dictionary-backed Japanese word segmentation.
Each `GlyphPlacement::construct` result identifies the containing construct, when present.

Callers can set OpenType features and variable-font axes globally or per span.
The optional `system-fonts` feature adds Fontique-backed OS discovery.
It passes weight, width, and slant to the matcher.
The `FontResource` records the matcher's default axes and synthetic styling.
Variation values merge by tag: global values first, then system-selected values, then span values.
The last value wins.
The cross-platform determinism contract applies to explicit font bytes.
After registration, a system face's copied bytes and rendering state are stable for that layout.
Discovery in a changed OS collection may select another face.

## Typed documents and all nine inline constructs

`DocumentBuilder` accepts UTF-8 byte ranges.
It controls the family, style, and language of spans.
Paragraph controls include measure, alignment, and first-line indent.
Widow policy and tab stops can also vary by range.
The builder supports mandatory, discretionary, and prohibited breaks alongside nine JLReq structures.
Ruby, emphasis marks, reference marks, and script annotations are ordinary strings shaped automatically.
Furawake balances its own sublines unless the caller supplies explicit splits.

Ruby uses `mono_ruby`, `group_ruby`, `jukugo_ruby`, or explicit `ruby` runs.
The builder exposes `縦中横` through `tate_chu_yoko` and `圏点` through `emphasis_dots`.
It exposes `割注` through `warichu` and `振分け` through `furawake`.
The `jidori` method represents `字取り`, and `reference_mark` represents `合印`.
The `script` method represents `添字`, and `formula` represents `数式`.

<!-- jlreq-example: document -->
```rust
use jlreq::{DocumentBuilder, FontLibrary, LayoutOptions, ScriptPosition, WritingMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("pass a font file, e.g. NotoSansJP-Regular.otf")?;

    let text = "漢字12 注記 割注 振分 字取 * H2O x+y";
    let mut document = DocumentBuilder::new(text);
    document.group_ruby(0..6, "かんじ")?;
    document.tate_chu_yoko(6..8)?;
    document.emphasis_dots(9..15, '・')?;
    document.warichu(16..22)?;
    document.furawake(23..29, 2, 1.0)?;
    document.jidori(30..36, 4)?;
    document.reference_mark(37..38, "※")?;
    document.script(39..42, "2", ScriptPosition::Subscript)?;
    document.formula(43..46)?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;
    let layout = jlreq::layout_document(
        &document.build()?,
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?.with_writing_mode(WritingMode::VerticalRl),
    )?;
    assert!(
        layout
            .lines()
            .iter()
            .all(|line| line.writing_mode() == WritingMode::VerticalRl)
    );
    Ok(())
}
```

The example uses byte ranges for its literal string.
Production code should derive ranges from the actual input.
Building is atomic: invalid relationships yield `LayoutError`.
Layout either returns a complete `TextLayout` or no result.

## Low-level composition

Applications that shape text can use `jlreq-core` directly or through `jlreq::core`.
The core preserves the pre-0.1.0 composition model.
Callers provide UTF-8 cluster ranges, integer advances, break opportunities, and typed constructs.
The core applies JLReq classification, kinsoku, and spacing.
It expands or reduces spacing, performs exact line search, and places the results.
It has no dependencies, I/O, or font parser, and supports `no_std + alloc` on Rust 1.85.
The `jlreq` library requires Rust 1.88.

The core has executable [`minimal`](../crates/jlreq-core/examples/minimal.rs), [`Composer`](../crates/jlreq-core/examples/composer.rs), and [`vertical`](../crates/jlreq-core/examples/vertical.rs) examples.
The [`jlreq` examples](../crates/jlreq/examples/) cover documents, vertical text, and paragraph styles.
They also cover policies, fallback, editing, and diagnostics.

## Conformance and development

`jlreq-conformance` is a binary-only NDJSON runner.
Protocol messages use `jlreq.conformance/1`.
The specification identifier is `jlreq-2020-08-11+unicode-17.0.0`.
The committed suite maintains the 100/106 conformance-rule ledger with independent OCaml and Racket engines.
The [protocol design](design/conformance.md) describes the generated 122,199-case censuses.
The project doesn't claim complete conformance to [Japanese Industrial Standards (JIS)](https://www.jisc.go.jp/qa/a3-1.html) X 4051.
Known layout defects remain under investigation, including the [captured geometry counterexample](https://github.com/P4suta/jlreq/issues/45).

```sh
just check          # formatting, lint, architecture, provenance, and repository hygiene
just test           # workspace tests and doctests
just examples       # compile and run every documentation example against a fixture font
just package        # isolated offline verification of all three .crate files
just release-check  # every reversible acceptance gate; never publishes
```

The [public API inventory](public-api.toml) records the frozen 0.1.x API.
The [error reference](error-codes.md) describes stable failures and diagnostics.
The [architecture](../ARCHITECTURE.md) and [security policy](../SECURITY.md) describe the design and resource bounds.
The [release procedure](RELEASING.md) documents the release handoff.
The [Japanese guide](guide.ja.md) provides more usage examples.

## License

The project is dual-licensed under [`MIT`](../LICENSE-MIT) or [`Apache-2.0`](../LICENSE-APACHE), at your option.
Those are the identifiers used in the repository's license expressions.
The `reuse lint` command checks the repository's [copyright and license declarations](https://reuse.software/tutorial/).
