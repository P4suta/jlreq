<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# jlreq-core

`jlreq-core` 0.1.0 is a dependency-free `no_std + alloc` Japanese line-composition engine for
already-shaped text. Callers provide UTF-8 byte ranges, cluster advances, and line-break
opportunities; the core returns integer logical placements without loading fonts, shaping,
running bidi, rendering, or discovering UAX #14 breaks.

Install with `cargo add jlreq-core` (MSRV 1.85, `no_std + alloc`, zero dependencies).
Most applications want the high-level [`jlreq`](https://crates.io/crates/jlreq) facade
instead; this crate is for engines that already shape their own text.

```rust
use jlreq_core::{Break, Cluster, Frame, Paragraph, ShapedText, Size, Style};

let source = "日本語組版";
let clusters = source.char_indices().map(|(start, ch)| {
    Cluster::new(start..start + ch.len_utf8(), 1_000)
});
let text = ShapedText::new(source, Size::square(1_000)?, Frame::FullEm, clusters)?;
let paragraph = Paragraph::builder(text, 4_000)
    .breaks(source.char_indices().skip(1).map(|(at, _)| Break::allowed(at)))
    .build()?;
let layout = jlreq_core::compose(&paragraph, &Style::book_2020())
    .expect("this small paragraph is within the default resource limits");

assert_eq!(layout.lines().len(), 2);
# Ok::<(), jlreq_core::InputError>(())
```

Composition returns either a complete exact `Layout` or a typed `ComposeError`; it never
returns a partial layout or silently changes search strategy. `CompositionLimits` bounds
clusters, break candidates, constructs, tab stops, and exact-search transitions. A
`Composer` retains scratch allocation across calls and remains reusable after an error.

The packaged, executable examples cover [minimal composition](examples/minimal.rs),
[Composer reuse and a resource refusal](examples/composer.rs), and
[vertical placement](examples/vertical.rs). The repository's
[ICU4X + HarfRust integration test](https://github.com/P4suta/jlreq/blob/main/crates/jlreq-conformance/tests/reference_integration.rs)
shows the intended segmentation/shaping boundary. See the
[repository guide](https://github.com/P4suta/jlreq) for scope and protocol details, or run
`cargo doc -p jlreq-core --open` for the API reference.
