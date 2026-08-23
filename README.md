# jlreq

`jlreq` is a Japanese line-composition engine for already-shaped text. It accepts UTF-8
source text, caller-unit cluster advances, break opportunities, and document structures;
it returns lines, placements, attachments, and stable diagnostics.

The public surface is deliberately limited to:

- the dependency-free `jlreq` Rust library (`no_std + alloc`), and
- the language-independent `jlreq-conformance` process protocol.

Font loading, shaping, UAX #14 segmentation, bidi resolution, rasterization, and drawing
remain the caller's responsibility.

This repository is an unreleased `0.0.0` development snapshot. It has not reached 0.1, and
neither package is publishable. The implementation exercises the candidate end-to-end
pipeline while its API and behavior remain free to change. The conformance inventory
currently reports zero mechanically implementable deferrals; three editorial and three
non-observable statements are classified with evidence rather than represented by empty
cases.

## Quick start

```rust
use jlreq::{Break, Cluster, Frame, Paragraph, ShapedText, Size, Style};

let source = "日本語組版";
let clusters = source.char_indices().map(|(start, ch)| {
    Cluster::new(start..start + ch.len_utf8(), 1_000)
});
let text = ShapedText::new(source, Size::square(1_000)?, Frame::FullEm, clusters)?;
let paragraph = Paragraph::builder(text, 4_000)
    .breaks(source.char_indices().skip(1).map(|(at, _)| Break::allowed(at)))
    .build()?;
let layout = jlreq::compose(&paragraph, &Style::book_2020());

for line in layout.lines() {
    for placement in line.clusters() {
        draw(placement);
    }
}
# Ok::<(), jlreq::InputError>(())
```

All input ranges are UTF-8 byte ranges. `ShapedText` owns the source and clusters;
`ParagraphBuilder` validates ranges, breaks, tabs, writing mode, widow control, and inline
constructs once. Composition is then infallible: an overfull or otherwise degraded result
still contains placements and a stable diagnostic.

Use `Composer` instead of the root `compose` function when composing repeatedly; it reuses
its search scratch space without lending it to the returned `Layout`.

## Scope

The library implements one paragraph pipeline:

```text
normalize → classify/space → lower constructs → optimize breaks → place
```

It accepts nine named inline constructs: ruby, tate-chu-yoko, emphasis dots, warichu,
furawake, jidori, reference marks, script complexes, and formulae. Ruby has explicit mono,
group, and jukugo runs. Horizontal and vertical paragraphs share the same logical inline
model; placements include the local writing mode and transform needed by a renderer.

The 22 alternative points derived from JLReq 2020 are dedicated enums in
`jlreq::style`. There is no public generic question/choice vocabulary and no public rule
identifier. `Style::default()` is permanently equal to `Style::jlreq_2020()`; dated book,
magazine, newspaper, and JIS-reading profiles are also available.

The specification identifier is
`jlreq-2020-08-11+unicode-17.0.0`. This includes the alternatives JLReq records from JIS X
4051; it is not a claim of complete JIS X 4051 conformance.

## Language-independent conformance

`jlreq-conformance` speaks NDJSON with an external engine process. Every message carries:

```json
{"protocol":"jlreq.conformance/1","spec":"jlreq-2020-08-11+unicode-17.0.0","id":"..."}
```

The fixed commands and exit codes are:

```text
jlreq-conformance list [SUITE.ndjson]
jlreq-conformance validate [SUITE.ndjson|-]
jlreq-conformance run ENGINE [SUITE.ndjson]

0  all cases conform / input validates
1  one or more result differences
2  input, protocol, or engine error
```

The package contains no library target. Its committed JSON Schema, built-in suite, and
`jlreq-sample-engine` executable form an end-to-end protocol example. See
[`docs/design/conformance.md`](docs/design/conformance.md).

[`crates/jlreq-conformance/tests/reference_integration.rs`](crates/jlreq-conformance/tests/reference_integration.rs)
keeps direct ICU4X line-break-byte-offset and HarfRust glyph-cluster adapters compiling and
running. Both dependencies are test-only; neither is a `jlreq` dependency or feature.

## Repository layout

| Path | Role |
| --- | --- |
| `crates/jlreq` | the only public Rust library |
| `crates/jlreq-conformance` | binary-only black-box runner and sample engine |
| `xtask` | specification generation and architectural gates |
| `spec/`, `data/` | vendored specification inputs, derived data, and provenance |

## Development

Development is test-first: write the observable failure, verify Red, implement the smallest
coherent behavior, verify Green, then refactor under the gates.

```sh
just check          # formatting, lint, architecture, provenance, and repository hygiene
just test           # workspace tests plus doctests
just ci             # all practical CI checks, including no_std and WASM
cargo run -p jlreq-conformance -- list
```

The candidate 1.0 names are tracked in [`docs/api-1.0.toml`](docs/api-1.0.toml). This is a
development control, not a released compatibility promise. The gate checks both missing and
extra exports, as well as all 22 typed Style mappings.

## License

Dual-licensed under [MIT](LICENSES/MIT.txt) or [Apache-2.0](LICENSES/Apache-2.0.txt), at
your option. The repository is [REUSE](https://reuse.software/)-compliant.
