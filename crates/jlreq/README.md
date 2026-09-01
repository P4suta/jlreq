<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# jlreq

`jlreq` turns UTF-8 text, in-memory TTF/OTF/TTC data, a font size, and a line extent into
renderer-independent, draw-ready glyph placements. It performs grapheme-aware font
fallback, HarfRust shaping, UAX #9 bidi resolution, UAX #14 line segmentation, and
deterministic Japanese composition through [`jlreq-core`](https://crates.io/crates/jlreq-core).

## Install

```sh
cargo add jlreq
```

```toml
[dependencies]
jlreq = "0.1"
```

The minimum supported Rust version is 1.88. No font ships with the crate: layout takes
font *bytes* you supply, so grab a TTF/OTF such as
[Noto Sans JP](https://fonts.google.com/noto/specimen/Noto+Sans+JP) to try the examples,
which read a font path from their first argument. Optional OS font discovery is behind a
feature: `cargo add jlreq --features system-fonts`.

## Quick start

<!-- jlreq-example: quickstart -->
```rust no_run
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

`TextLayout` owns every font resource referenced by its glyphs, so it can be passed to a
renderer without retaining `FontLibrary`. Retained font IDs can be sparse; resolve each
`GlyphPlacement::font_id` with `TextLayout::font`. Glyphs preserve the original UTF-8 byte
range, draw origin, size, effective variation axes, advances, transform, bidi level, cell
bounds, and visual draw order. `FontResource` also preserves system-selected default axes,
synthetic bold/skew state, and em-relative design metrics (`FontResource::metrics`) for
underline, strikethrough, and baseline work.
Rasterization, drawing, PDF serialization, and GPU integration remain renderer concerns.

Glyph, line, and layout `bounds` are physical layout-cell bounds rather than ink bounds.
They include whitespace and annotation cells. Outline ink bounds remain a renderer concern.

Use `LayoutEngine` for repeated work, `DocumentBuilder` for spans, paragraph styles,
explicit break control, ruby and the other eight typed inline constructs, and
`jlreq::core` when text is already shaped and the caller wants exact control over clusters
and break opportunities.

## Composition policy (組版ポリシー)

Every JLReq 2020 alternative — kinsoku strictness, hanging punctuation, ruby overhang,
adjustment order, and the rest — is one typed `Style` value, applied through
`LayoutOptions::with_style`. `Style::jlreq_2020()` is the default reading of the
specification; `book_2020()`, `magazine_2020()`, `newspaper_2020()`, and
`jis_reading_2020()` are the other published profiles, and `StyleBuilder` overrides any
single choice.

```rust no_run
use jlreq::{LayoutOptions, Style};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = LayoutOptions::try_new(240.0, 16.0)?.with_style(Style::book_2020());
    let _ = options.style();
    Ok(())
}
```

## Typed documents and vertical layout

All nine JLReq inline structures are typed builder calls; annotation strings for ruby,
emphasis, reference marks, and scripts are shaped automatically. Furawake balances its
sublines on its own — supply `mandatory_break` positions inside the range only when you
want explicit splits.

<!-- jlreq-example: document -->
```rust no_run
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

## Paragraph styles

`ParagraphStyle` overrides the measure, alignment, JLReq policy, first-line indent, widow
policy, or tab stops for every paragraph a range contains — indented body text, a
narrower quotation, a centered heading — while `LayoutOptions` carries the document-wide
defaults (`with_first_line_indent`, `with_widow`, `with_tab_stops`).

```rust no_run
use jlreq::{Alignment, DocumentBuilder, FontLibrary, LayoutOptions, ParagraphStyle};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("pass a font file, e.g. NotoSansJP-Regular.otf")?;

    let text = "見出し\n本文の段落です。";
    let mut document = DocumentBuilder::new(text);
    document.paragraph_style(
        0..9,
        ParagraphStyle::new().with_alignment(Alignment::Center),
    )?;
    document.paragraph_style(
        10..text.len(),
        ParagraphStyle::new().with_first_line_indent(16.0)?,
    )?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;
    let layout = jlreq::layout_document(
        &document.build()?,
        &fonts,
        LayoutOptions::try_new(320.0, 16.0)?,
    )?;
    assert!(layout.lines().first().is_some_and(|line| {
        line.is_first_in_paragraph()
    }));
    Ok(())
}
```

## Editing geometry

`caret_rect` requires the affinity returned by hit testing. This preserves the selected
visual edge at wraps, paragraph breaks, and bidi boundaries. The layout also carries the
rest of an editor's needs: visual caret motion (`next_visual_caret`), line-to-line motion
(`caret_next_line`), grapheme/word/sentence segmentation with dictionary-backed Japanese
word boundaries (`word_range_at`), per-glyph construct attribution
(`GlyphPlacement::construct`), and filled per-line selection highlighting
(`selection_rects_filled`).

```rust no_run
use jlreq::{FontLibrary, LayoutOptions, Point};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("pass a font file, e.g. NotoSansJP-Regular.otf")?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;
    let layout = jlreq::layout(
        "日本語 abc العربية",
        &fonts,
        LayoutOptions::try_new(160.0, 16.0)?,
    )?;

    let hit = layout.hit_test(Point::try_new(24.0, 12.0)?);
    if let Some(caret) = layout.caret_rect(hit.byte_offset(), hit.affinity()) {
        let _physical_caret = caret.as_26_6();
    }
    let _word = layout.word_range_at(hit.byte_offset());
    let _next = layout.next_visual_caret(hit.byte_offset(), hit.affinity());
    let _visual_runs = layout.selection_rects(0.."日本語".len());
    Ok(())
}
```

## Diagnostics

Recoverable conditions accompany a complete layout; invalid input and exhausted limits
return `LayoutError` without a partial result. `font.missing-glyph` reports a grapheme no
registered face covers, and `font.unknown-family` reports a span family no registered face
declares.

```rust no_run
use jlreq::{FontLibrary, LayoutOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("pass a font file, e.g. NotoSansJP-Regular.otf")?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;
    let layout = jlreq::layout(
        "missing: \u{10ffff}",
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?,
    )?;
    for diagnostic in layout.diagnostics() {
        eprintln!(
            "{} {:?} {:?}: {}",
            diagnostic.code(),
            diagnostic.severity(),
            diagnostic.range(),
            diagnostic.message(),
        );
    }
    Ok(())
}
```

The optional `system-fonts` feature exposes Fontique-backed OS font discovery. Layout from
explicit font bytes is deterministic across supported operating systems; selection from a
changing system collection is intentionally outside that guarantee. Global values,
system-selected defaults, and span values are merged per variation tag in that order, with
the last value winning.

Licensed under MIT or Apache-2.0, at your option.
