// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Ask why a layout came out as it did, instead of reading coordinates and guessing.

use std::error::Error;

use jlreq::trace::{Categories, DocumentTrace, Fact};
use jlreq::{FontLibrary, LayoutEngine, LayoutOptions, ResourceLimits};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a TTF, OTF, or TTC path")?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(path)?)?;
    let mut engine = LayoutEngine::new();
    let text = "日本語の組版、その理由。\n二つ目の段落。";

    // The whole trace, rendered: one decision per line, facade and core in one
    // stream. `layout` and `layout_traced` run one body, so this explains what
    // an untraced call does rather than a second code path.
    let mut trace = DocumentTrace::new();
    let layout = engine.layout_traced(text, &fonts, LayoutOptions::try_new(240.0, 16.0)?, &mut trace)?;
    println!("{} line(s), {} event(s)", layout.lines().len(), trace.events().len());
    print!("{trace}");

    // Or ask one question. Narrowing the families keeps the answer short: this
    // records nothing but face decisions, in both channels.
    let mut faces = DocumentTrace::with_categories(
        Categories::FACES,
        jlreq::core::trace::Categories::NONE,
    );
    let _ = engine.layout_traced(text, &fonts, LayoutOptions::try_new(240.0, 16.0)?, &mut faces)?;
    for event in faces.events() {
        let source = &text[event.site().bytes()];
        match event.fact() {
            Fact::FaceChosen {
                family,
                position,
                candidates,
                ..
            } => println!(
                "{source:?} was covered by {family:?}, candidate {position} of {candidates}"
            ),
            Fact::FaceFallback {
                family, candidates, ..
            } => println!(
                "{source:?} is covered by none of the {candidates} candidate(s); \
                 {family:?} kept its .notdef"
            ),
            other => println!("{source:?}: {kind}", kind = other.kind()),
        }
    }

    // A diagnostic says a notable thing happened; the trace says what produced
    // it. Here the measure is too narrow for one cluster, so the line reports
    // `layout.overfull` — and the `line.fit` beside it gives the arithmetic.
    let mut overfull = DocumentTrace::new();
    let narrow = LayoutOptions::try_new(4.0, 16.0)?;
    let tight = engine.layout_traced("組版", &fonts, narrow, &mut overfull)?;
    for diagnostic in tight.diagnostics() {
        println!("{}: {}", diagnostic.code(), diagnostic.message());
    }
    for event in overfull.events() {
        if event.kind() == "line.fit" {
            println!("  because {event}");
        }
    }

    // A hard error keeps its reasoning too: the trace is filled as the call
    // proceeds and survives the `?` that ends it.
    let mut refused = DocumentTrace::new();
    let starved = LayoutOptions::try_new(240.0, 16.0)?
        .with_limits(ResourceLimits::default().with_max_glyphs(1));
    match engine.layout_traced("組版", &fonts, starved, &mut refused) {
        Ok(_) => println!("unexpectedly fit within one glyph"),
        Err(error) => println!(
            "error {code} after {count} recorded decision(s)",
            code = error.code(),
            count = refused.events().len()
        ),
    }
    Ok(())
}
