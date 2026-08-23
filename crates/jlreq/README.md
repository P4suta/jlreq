<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# jlreq

`jlreq` is a dependency-free `no_std + alloc` Japanese line-composition engine for
already-shaped text. Callers provide UTF-8 byte ranges, cluster advances, and line-break
opportunities; jlreq returns logical placements and diagnostics without loading fonts,
shaping, running bidi, or drawing.

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

See the [repository guide](https://github.com/P4suta/jlreq) for the unreleased development
status, scope, shaping and segmentation integrations, the language-independent conformance
protocol, and development policy. Generate API documentation locally with
`cargo doc -p jlreq --open`.
