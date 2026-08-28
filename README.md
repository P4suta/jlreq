# jlreq

`jlreq` lays out Japanese and mixed-script UTF-8 text from in-memory fonts. Give it text,
font bytes, a font size, and a line extent; it returns visual-order glyphs with physical
coordinates, source ranges, transforms, bidi levels, and the font resources needed to draw
them.

The 0.1.0 workspace has three deliberately separate products:

| Product | Shortest path for |
| --- | --- |
| `jlreq` | applications that want shaping, fallback, bidi, line breaking, and composition in one call |
| `jlreq-core` | engines that already have shaped clusters and require dependency-free `no_std + alloc` composition |
| `jlreq-conformance` | implementations validating protocol-v1 behavior against the black-box suite |

This tree is prepared up to the last reversible step before publication. It creates and
verifies three crate archives, six target-specific conformance binary archives, checksums,
SBOMs, and attestations in CI, but it does not upload a crate, choose the release date,
create `v0.1.0`, or create a GitHub Release.

## Quick start

This same example is the `jlreq` crate's compiled doctest:

```rust,no_run
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

`TextLayout` owns every `FontResource` referenced by its glyphs. A renderer therefore needs
only the returned layout: use `font_id` to select bytes and TTC face, draw `glyph_id` at the
physical origin plus offsets, and apply `GlyphTransform`. Rasterization, GPU upload, PDF
encoding, and drawing are intentionally outside the crate.

Use `LayoutEngine` instead of `jlreq::layout` for batches. It reuses parsed fonts, shaping
data, Unicode services, and core-composer scratch space, remains reusable after an error,
and produces bit-identical results to the one-shot call.

## Fonts, fallback, vertical text, and bidi

`FontLibrary::register_face` records a TTF/OTF or a TTC face with family and style metadata.
The primary face supplies `.notdef`; `set_fallback_order` sets an explicit priority. The
first font that covers an entire extended grapheme (including a variation sequence) wins.
If none does, the source range is retained, the primary `.notdef` is emitted, and
`font.missing-glyph` is reported as a diagnostic.

```rust,no_run
# fn configure(japanese: Vec<u8>, latin: Vec<u8>) -> Result<(), jlreq::LayoutError> {
use jlreq::{BaseDirection, FontLibrary, LayoutOptions, WritingMode};

let mut fonts = FontLibrary::new();
let jp = fonts.register_font(japanese)?;
let latin = fonts.register_font(latin)?;
fonts.set_primary(jp)?;
fonts.set_fallback_order([jp, latin])?;

let options = LayoutOptions::try_new(320.0, 16.0)?
    .writing_mode(WritingMode::VerticalRl)
    .base_direction(BaseDirection::Auto);
let layout = jlreq::layout("日本語 Latin العربية", &fonts, options)?;
assert!(layout.glyphs().all(|glyph| !glyph.source_range().is_empty()));
# Ok(())
# }
```

Horizontal and vertical results use physical coordinates. Latin rotation, upright CJK,
tate-chu-yoko, per-line UAX #9 visual reordering, hit testing, caret rectangles, and
selection rectangles therefore require no logical-to-physical conversion in the caller.
OpenType features and variable-font axes can be set globally or per span. The optional
`system-fonts` feature adds Fontique-backed OS discovery; only explicit bytes carry the
cross-platform determinism guarantee.

## Typed documents and all nine inline constructs

`DocumentBuilder` accepts UTF-8 byte ranges. It also controls span family/style/language,
mandatory and prohibited breaks, and nine JLReq structures. Ruby, emphasis marks,
reference marks, and script annotations are ordinary strings and are shaped automatically.

| Structure | Builder method |
| --- | --- |
| mono/group/jukugo ruby | `mono_ruby`, `group_ruby`, `jukugo_ruby`, or explicit `ruby` runs |
| 縦中横 | `tate_chu_yoko` |
| 圏点 | `emphasis_dots` |
| 割注 | `warichu` |
| 振分け | `furawake` |
| 字取り | `jidori` |
| 合印 | `reference_mark` |
| 添字 | `script` |
| 数式 | `formula` |

```rust,no_run
# fn document(font_bytes: Vec<u8>) -> Result<(), jlreq::LayoutError> {
use jlreq::{DocumentBuilder, FontLibrary, LayoutOptions, ScriptPosition};

let text = "漢字12 注記 振分 字取 * H2O x+y";
let mut doc = DocumentBuilder::new(text);
doc.group_ruby(0..6, "かんじ")?;
doc.tate_chu_yoko(6..8)?;
doc.emphasis_dots(9..15, '・')?;
doc.warichu(16..22)?;
doc.furawake(23..29, 2, 1.0)?;
doc.jidori(30..36, 4)?;
doc.reference_mark(37..38, "※")?;
doc.script(39..42, "2", ScriptPosition::Subscript)?;
doc.formula(43..46)?;

let mut fonts = FontLibrary::new();
fonts.register_font(font_bytes)?;
let layout = jlreq::layout_document(
    &doc.build()?,
    &fonts,
    LayoutOptions::try_new(240.0, 16.0)?,
)?;
assert!(layout.glyphs().count() > 0);
# Ok(())
# }
```

The example ranges are illustrative; production code should derive ranges from the actual
string. Building is atomic: invalid relationships yield `LayoutError`, and layout either
returns a complete `TextLayout` or no result.

## Low-level composition

Applications that already shape text can depend on `jlreq-core` directly, or reach the
same API through `jlreq::core`. It preserves the pre-0.1.0 composition model: callers
provide UTF-8 cluster ranges, integer advances, break opportunities, and typed constructs;
the core applies JLReq classification, kinsoku, spacing, adjustment, exact line search, and
placement. It has no dependencies, no I/O, no font parser, and supports `no_std + alloc`
on MSRV 1.85. The facade uses MSRV 1.88.

See the executable [`minimal`](crates/jlreq-core/examples/minimal.rs),
[`Composer`](crates/jlreq-core/examples/composer.rs), and
[`vertical`](crates/jlreq-core/examples/vertical.rs) examples for that layer.

## Conformance and development

`jlreq-conformance` is a binary-only NDJSON runner. Protocol messages retain
`jlreq.conformance/1` and specification identifier
`jlreq-2020-08-11+unicode-17.0.0`. The committed suite and independent OCaml/Racket engines
keep the current 100/106 conformance-rule ledger and generated 122,199-case censuses.
See [the protocol design](docs/design/conformance.md).

```sh
just check          # formatting, lint, architecture, provenance, and repository hygiene
just test           # workspace tests and doctests
just package        # isolated offline verification of all three .crate files
just release-check  # every reversible acceptance gate; never publishes
```

The exact 0.1.x public API is frozen in
[`docs/public-api.toml`](docs/public-api.toml); stable failures and diagnostics are in
[`docs/error-codes.md`](docs/error-codes.md). Architecture, resource bounds, and the release
handoff are documented in [ARCHITECTURE.md](ARCHITECTURE.md), [SECURITY.md](SECURITY.md),
and [docs/RELEASING.md](docs/RELEASING.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. The
repository is [REUSE](https://reuse.software/)-compliant.
