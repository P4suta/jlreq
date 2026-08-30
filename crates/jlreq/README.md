# jlreq

`jlreq` turns UTF-8 text, in-memory TTF/OTF/TTC data, a font size, and a line extent into
renderer-independent, draw-ready glyph placements. It performs grapheme-aware font
fallback, HarfRust shaping, UAX #9 bidi resolution, UAX #14 line segmentation, and
deterministic Japanese composition through [`jlreq-core`](https://crates.io/crates/jlreq-core).

```rust
# fn quick_start(font_bytes: Vec<u8>) -> Result<(), jlreq::LayoutError> {
use jlreq::{FontLibrary, LayoutOptions};

let mut fonts = FontLibrary::new();
fonts.register_font(font_bytes)?;

let options = LayoutOptions::try_new(240.0, 16.0)?;
let layout = jlreq::layout("日本語組版 — draw-ready glyphs", &fonts, options)?;

for glyph in layout.glyphs() {
    if let Some(font) = layout.font(glyph.font_id()) {
        renderer_draw(
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
# fn renderer_draw(
#     _: &[u8],
#     _: u32,
#     _: u32,
#     _: jlreq::Point,
#     _: i32,
#     _: &[jlreq::FontVariation],
#     _: jlreq::FontSynthesis,
#     _: jlreq::GlyphTransform,
# ) {}
# Ok(())
# }
```

`TextLayout` owns every font resource referenced by its glyphs, so it can be passed to a
renderer without retaining `FontLibrary`. Retained font IDs can be sparse; resolve each
`GlyphPlacement::font_id` with `TextLayout::font`. Glyphs preserve the original UTF-8 byte
range, draw origin, size, effective variation axes, advances, transform, bidi level, cell
bounds, and visual draw order. `FontResource` also preserves system-selected default axes
and synthetic bold/skew state.
Rasterization, drawing, PDF serialization, and GPU integration remain renderer concerns.

Glyph, line, and layout `bounds` are physical layout-cell bounds rather than ink bounds.
They include whitespace and annotation cells. Outline ink bounds remain a renderer concern.

Use `LayoutEngine` for repeated work, `DocumentBuilder` for spans, explicit break control,
ruby and the other eight typed inline constructs, and `jlreq::core` when text is already
shaped and the caller wants exact control over clusters and break opportunities.

## Typed vertical layout

```rust,no_run
# fn vertical(font_bytes: Vec<u8>) -> Result<(), jlreq::LayoutError> {
use jlreq::{DocumentBuilder, FontLibrary, LayoutOptions, ScriptPosition, WritingMode};

let text = "漢字12 注記 割注 振分 字取 * H2O x+y";
let mut document = DocumentBuilder::new(text);
document.group_ruby(0..6, "かんじ")?;
document.tate_chu_yoko(6..8)?;
document.emphasis_dots(9..15, '・')?;
document.warichu(16..22)?;
document.furawake(23..29, 2, 1.0)?;
document.mandatory_break(26)?;
document.jidori(30..36, 4)?;
document.reference_mark(37..38, "※")?;
document.script(39..42, "2", ScriptPosition::Subscript)?;
document.formula(43..46)?;

let mut fonts = FontLibrary::new();
fonts.register_font(font_bytes)?;
let layout = jlreq::layout_document(
    &document.build()?,
    &fonts,
    LayoutOptions::try_new(240.0, 16.0)?
        .writing_mode(WritingMode::VerticalRl),
)?;
assert!(layout.lines().iter().all(|line| {
    line.writing_mode() == WritingMode::VerticalRl
}));
# Ok(())
# }
```

## Editing geometry

`caret_rect` requires the affinity returned by hit testing. This preserves the selected
visual edge at wraps, paragraph breaks, and bidi boundaries.

```rust,no_run
# fn editing(font_bytes: Vec<u8>) -> Result<(), jlreq::LayoutError> {
use jlreq::{FontLibrary, LayoutOptions, Point};

let mut fonts = FontLibrary::new();
fonts.register_font(font_bytes)?;
let layout = jlreq::layout(
    "日本語 abc العربية",
    &fonts,
    LayoutOptions::try_new(160.0, 16.0)?,
)?;
let hit = layout.hit_test(Point::try_new(24.0, 12.0)?);
if let Some(caret) = layout.caret_rect(hit.byte_offset(), hit.affinity()) {
    let _physical_caret = caret.as_26_6();
}
let _visual_runs = layout.selection_rects(0.."日本語".len());
# Ok(())
# }
```

## Diagnostics

Recoverable conditions accompany a complete layout; invalid input and exhausted limits
return `LayoutError` without a partial result.

```rust,no_run
# fn diagnostics(font_bytes: Vec<u8>) -> Result<(), jlreq::LayoutError> {
use jlreq::{FontLibrary, LayoutOptions};

let mut fonts = FontLibrary::new();
fonts.register_font(font_bytes)?;
let layout = jlreq::layout("missing: \u{10ffff}", &fonts, LayoutOptions::try_new(240.0, 16.0)?)?;
for diagnostic in layout.diagnostics() {
    eprintln!(
        "{} {:?} {:?}: {}",
        diagnostic.code(),
        diagnostic.severity(),
        diagnostic.range(),
        diagnostic.message(),
    );
}
# Ok(())
# }
```

The optional `system-fonts` feature exposes Fontique-backed OS font discovery. Layout from
explicit font bytes is deterministic across supported operating systems; selection from a
changing system collection is intentionally outside that guarantee. Global values,
system-selected defaults, and span values are merged per variation tag in that order, with
the last value winning.

Licensed under MIT or Apache-2.0, at your option.
