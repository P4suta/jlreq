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
    let font = &layout.fonts()[glyph.font_id().get() as usize];
    renderer_draw(
        font.bytes(),
        font.face_index(),
        glyph.glyph_id(),
        glyph.origin(),
        glyph.transform(),
    );
}
# fn renderer_draw(
#     _: &[u8], _: u32, _: u32, _: jlreq::Point, _: jlreq::GlyphTransform,
# ) {}
# Ok(())
# }
```

`TextLayout` owns every font resource referenced by its glyphs, so it can be passed to a
renderer without retaining `FontLibrary`. Glyphs preserve the original UTF-8 byte range,
physical origin, advances and offsets, transform, bidi level, and visual draw order.
Rasterization, drawing, PDF serialization, and GPU integration remain renderer concerns.

Use `LayoutEngine` for repeated work, `DocumentBuilder` for spans, explicit break control,
ruby and the other eight typed inline constructs, and `jlreq::core` when text is already
shaped and the caller wants exact control over clusters and break opportunities.

The optional `system-fonts` feature exposes Fontique-backed OS font discovery. Layout from
explicit font bytes is deterministic across supported operating systems; selection from a
changing system collection is intentionally outside that guarantee.

Licensed under MIT or Apache-2.0, at your option.
