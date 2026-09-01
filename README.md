<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

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

## Install

```sh
cargo add jlreq
```

```toml
[dependencies]
jlreq = "0.1"
```

The `jlreq` facade requires Rust 1.88; `jlreq-core` and `jlreq-conformance` build on
1.85. No font ships with the crates — layout takes font *bytes* you supply, so grab a
TTF/OTF such as [Noto Sans JP](https://fonts.google.com/noto/specimen/Noto+Sans+JP) to
try the examples, which read a font path from their first argument. OS font discovery is
opt-in: `cargo add jlreq --features system-fonts`. 日本語の利用ガイドは
[`docs/guide.ja.md`](docs/guide.ja.md) にあります。

## Quick start

This is the same example the `jlreq` crate compiles as a doctest, verbatim — a repository
gate keeps the two copies identical and runs this one against a fixture font:

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

`TextLayout` owns every `FontResource` referenced by its glyphs. Retained IDs can be sparse,
so renderers resolve them with `TextLayout::font`, not slice indexing. Draw `glyph_id` at
`draw_origin`, instantiate it with the returned size and effective variations, apply the
font's synthetic emboldening/skew and then `GlyphTransform`. `FontResource::metrics`
supplies em-relative ascent, descent, x-height, cap height, and underline geometry for
decorations. Rasterization, GPU upload, PDF encoding, and drawing are intentionally
outside the crate.

`GlyphPlacement::cell_bounds`, `TextLine::bounds`, and `TextLayout::bounds` are physical
layout-cell boundaries. They deliberately retain whitespace and annotation cells. They are
not glyph ink bounds; a rasterizer must derive ink bounds from the selected outline and
variation instance when clipping or painting decorations.

Use `LayoutEngine` instead of `jlreq::layout` for batches. It reuses parsed fonts, shaping
data, Unicode services, and core-composer scratch space, remains reusable after an error,
and produces bit-identical results to the one-shot call.

## Composition policy (組版ポリシー)

Every JLReq 2020 alternative is one typed `Style` value: `Style::jlreq_2020()` (the
default), `book_2020()`, `magazine_2020()`, `newspaper_2020()`, and `jis_reading_2020()`
are the published profiles, and `StyleBuilder` overrides any single choice — kinsoku
strictness, hanging punctuation, ruby overhang, adjustment order, and the other typed
alternatives listed in [`docs/public-api.toml`](docs/public-api.toml). Apply one with
`LayoutOptions::with_style`, or per paragraph with `ParagraphStyle::with_style`.

## Fonts, fallback, vertical text, and bidi

`FontLibrary::register_font` derives the family name from the font's own `name` table, so
a span's family request matches without extra registration metadata;
`FontLibrary::register_face` records explicit family and style metadata for a TTF/OTF or
TTC face. The primary face supplies `.notdef`; `set_fallback_order` sets an explicit
priority. The first font that covers an entire extended grapheme (including a variation
sequence) wins. If none does, the source range is retained, the primary `.notdef` is
emitted, and `font.missing-glyph` is reported as a diagnostic; a span family nothing
declares reports `font.unknown-family`.

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
    let layout = jlreq::layout("日本語 Latin العربية", &fonts, options)?;
    assert!(layout.glyphs().all(|glyph| !glyph.source_range().is_empty()));
    Ok(())
}
```

Horizontal and vertical results use physical coordinates. Latin rotation, upright CJK,
tate-chu-yoko, per-line UAX #9 visual reordering, hit testing, caret rectangles, and
selection rectangles therefore require no logical-to-physical conversion in the caller.
The same layout carries the editing primitives an editor needs: visual and line-to-line
caret motion, grapheme boundaries, dictionary-backed Japanese word segmentation, and
per-glyph construct attribution. OpenType features and variable-font axes can be set
globally or per span. The optional `system-fonts` feature adds Fontique-backed OS
discovery, passes weight/width/slant to the matcher, and records its default axes and
synthetic styling in `FontResource`. Global, system-selected, and span variation values
are merged by tag in that order, with the last value winning. Only explicit bytes carry
the cross-platform determinism guarantee; once a system face is registered, its copied
bytes and recorded rendering state are stable for that layout, but repeating discovery
against a changed OS collection may select another face.

## Typed documents and all nine inline constructs

`DocumentBuilder` accepts UTF-8 byte ranges. It also controls span family/style/language,
paragraph styles (measure, alignment, first-line indent, widow policy, and tab stops per
range), mandatory, discretionary, and prohibited breaks, and nine JLReq structures. Ruby,
emphasis marks, reference marks, and script annotations are ordinary strings and are
shaped automatically; furawake balances its own sublines unless explicit splits are
supplied.

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
[`vertical`](crates/jlreq-core/examples/vertical.rs) examples for that layer, and
[`crates/jlreq/examples/`](crates/jlreq/examples/) for runnable facade walkthroughs of
documents, vertical text, paragraph styles, policies, fallback, editing, and diagnostics.

## Conformance and development

`jlreq-conformance` is a binary-only NDJSON runner. Protocol messages retain
`jlreq.conformance/1` and specification identifier
`jlreq-2020-08-11+unicode-17.0.0`. The committed suite and independent OCaml/Racket engines
keep the current 100/106 conformance-rule ledger and generated 122,199-case censuses.
See [the protocol design](docs/design/conformance.md).

```sh
just check          # formatting, lint, architecture, provenance, and repository hygiene
just test           # workspace tests and doctests
just examples       # compile and run every documentation example against a fixture font
just package        # isolated offline verification of all three .crate files
just release-check  # every reversible acceptance gate; never publishes
```

The exact 0.1.x public API is frozen in
[`docs/public-api.toml`](docs/public-api.toml); stable failures and diagnostics are in
[`docs/error-codes.md`](docs/error-codes.md). Architecture, resource bounds, and the release
handoff are documented in [ARCHITECTURE.md](ARCHITECTURE.md), [SECURITY.md](SECURITY.md),
and [docs/RELEASING.md](docs/RELEASING.md). A Japanese usage guide is available at
[`docs/guide.ja.md`](docs/guide.ja.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. The
repository is [REUSE](https://reuse.software/)-compliant.
