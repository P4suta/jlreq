# jlreq

`jlreq` is a Japanese line-composition engine for already-shaped text. It accepts UTF-8
source text, caller-unit cluster advances, break opportunities, and document structures;
it returns lines, placements, attachments, and stable diagnostics.

The public surface is deliberately limited to:

- the dependency-free `jlreq` Rust library (`no_std + alloc`), and
- the language-independent `jlreq-conformance` process protocol.

Font loading, shaping, UAX #14 segmentation, bidi resolution, rasterization, and drawing
remain the caller's responsibility.

This tree is prepared as version 0.1.0: both crate archives, binaries, release metadata,
and verification workflows can be produced without publishing. No crate upload, tag, or
GitHub Release is performed by the preparation gates. Within 0.1.x, the public Rust surface
recorded in `docs/public-api.toml`, protocol v1, stable error codes, and MSRV 1.85 are
compatibility contracts.

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
let layout = jlreq::compose(&paragraph, &Style::book_2020())
    .expect("this small paragraph is within the default resource limits");
assert_eq!(layout.lines().len(), 2);
# Ok::<(), jlreq::InputError>(())
```

All input ranges are UTF-8 byte ranges. `ShapedText` owns the source and clusters;
`ParagraphBuilder` validates ranges, breaks, tabs, writing mode, widow control, and inline
constructs once. Composition returns a complete exact `Layout` or a typed `ComposeError`.
It never returns a partial layout, approximates a placement, or silently falls back to
first-fit. Fit conditions that remain valid but cannot be improved, such as an overfull
line, still produce a complete layout with a stable diagnostic.

Use `Composer` instead of the root `compose` function when composing repeatedly; it reuses
its search scratch space without lending it to the returned `Layout`, supports explicit
`CompositionLimits`, and remains reusable after a resource error. See the executable
[`minimal`](crates/jlreq/examples/minimal.rs),
[`Composer`](crates/jlreq/examples/composer.rs), and
[`vertical`](crates/jlreq/examples/vertical.rs) examples. The
[`reference_integration`](crates/jlreq-conformance/tests/reference_integration.rs) test
connects ICU4X byte break offsets and HarfRust glyph clusters at the intended caller seam.

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

All commands accept `--help`, `--version`, `--verbose`, `--timeout-seconds`,
`--max-message-bytes`, `--max-suite-bytes`, and `--max-cases`. Defaults are 30 seconds
without communication, 1 MiB per message, 256 MiB per suite, and 200,000 cases. Requests
and responses stream concurrently; responses may arrive in any order and are matched by a
unique `id`. Duplicate, unknown, missing, or extra responses are protocol errors.

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
| `engines/` | independent, non-product reference engines that speak protocol v1 |

## Independent reference engines

Protocol independence is only a claim until a second implementation tests it.
[`engines/ocaml/`](engines/ocaml/README.md) is a from-scratch OCaml implementation of
protocol v1, built directly from `spec/` and barred from reading `crates/jlreq/src/`;
`engines/racket/` follows the same rule. Neither is a Cargo workspace member or a product;
see [`docs/design/conformance.md`](docs/design/conformance.md#independent-reference-engines)
and [ADR 0024](docs/adr/0024-independent-reference-engines.md).

## Development

Development is test-first: write the observable failure, verify Red, implement the smallest
coherent behavior, verify Green, then refactor under the gates.

```sh
just check          # formatting, lint, architecture, provenance, and repository hygiene
just test           # workspace tests plus doctests
just ci             # all practical CI checks, including no_std and WASM
cargo run -p jlreq-conformance -- list
```

The 0.1.0 names and release-line contract are tracked in
[`docs/public-api.toml`](docs/public-api.toml). The network-free API gate checks missing and
extra exports plus all 22 typed Style mappings. Starting with the next 0.1.x candidate, the
required semver job also compares the complete rustdoc API with the latest published jlreq
release. Stable error and diagnostic codes are listed in
[`docs/error-codes.md`](docs/error-codes.md).

## License

Dual-licensed under [MIT](LICENSES/MIT.txt) or [Apache-2.0](LICENSES/Apache-2.0.txt), at
your option. The repository is [REUSE](https://reuse.software/)-compliant.
