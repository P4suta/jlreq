// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Every construct the builder offers, at every length from one cluster to
//! five, in both writing modes, asked whether its geometry holds.
//!
//! # Why a matrix
//!
//! Because a construct tested at one length is a construct tested at one
//! length. Every tate-chu-yoko fixture in this workspace — the geometry corpus,
//! the `shared-space` golden, the SVG example, `jlreq-core`'s own public test
//! and the conformance case — uses exactly two members, which is the single
//! count at which that construct's placement defect cancels. Furawake had no
//! geometric test at any length, and is wrong at every length it can hold.
//!
//! Sweeping the length is what found both. The sweep costs a fraction of a
//! second and it is the only thing here that asks a construct a question its
//! author did not choose.
//!
//! # What it pins
//!
//! The set of `(construct, clusters, writing mode)` triples whose layout
//! `jlreq::verify` calls unsound. Soundness, not coordinates: the coordinates
//! belong to `construct_geometry.rs`, which states them exactly for the three
//! constructs that once had defects. A row here says only *that* a
//! combination is broken, which survives a change of fixture or measure and
//! fails the moment one is fixed or a new one breaks.
//!
//! The measure is asked twice, wide enough to hold the paragraph and narrow
//! enough to wrap it, and a construct whose soundness depends on which is a
//! finding in itself — so that is asserted rather than collapsed.
//!
//! # What it found
//!
//! Three defects, all corrected in
//! [ADR 0030](../../../docs/adr/0030-a-construct-is-centred-in-its-line.md), and
//! all of one shape: a construct was positioned against **the paragraph's em**
//! rather than against the line's own block extent, which the line then grew
//! without re-centring what it grew around.
//!
//! - **Warichu.** Set at full size in the one em its line reserves for two
//!   lanes, so the pair was twice what the line held. Broken at every length.
//! - **Furawake.** `place_furawake_segment` centred the segment inside
//!   `paragraph.text.size().block()` — one em — while the line reserves one em
//!   per column, so every lane landed half the surplus early. Broken at every
//!   length that fills every column, in both writing modes, and nothing in the
//!   workspace had a geometric test for one.
//! - **Tate-chu-yoko.** Displaced by `(members − 2) × advance / 2`, which is
//!   zero at the two members every fixture in the workspace used.
//!
//! Everything else — mono, group and jukugo ruby, emphasis dots, jidori,
//! reference marks, superscripts, subscripts and formulas — is sound at every
//! length asked and in both modes, and this file is what keeps that true. Asked
//! is not the same as sound: `REFUSED` names the lengths a construct declines to
//! lay out at all, and those were never measured.

use std::sync::Arc;

use jlreq::{DocumentBuilder, FontLibrary, FontStyle, LayoutOptions, ScriptPosition, WritingMode};

/// Attach one construct to the `clusters` kanji that sit at bytes 6.. in the
/// fixture text.
///
/// An `Err` fails the sweep: a construct that cannot be *declared* at a length
/// the matrix claims to cover is a hole in the matrix, not a skip. A construct
/// that declares but cannot be laid out is a refusal, and `REFUSED` pins those.
type Attach = fn(&mut DocumentBuilder, usize) -> Result<(), jlreq::LayoutError>;

/// Byte offset of the first kanji the construct is attached to: `前と`.
const START: usize = 6;

fn fixture() -> Result<FontLibrary, Box<dyn std::error::Error>> {
    let mut fonts = FontLibrary::new();
    fonts.register_face(
        Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF),
        0,
        "Noto Sans JP",
        FontStyle::default(),
    )?;
    Ok(fonts)
}

fn constructs() -> Vec<(&'static str, Attach)> {
    vec![
        ("mono-ruby", |builder, n| {
            builder.mono_ruby(range(n), reading(n)).map(|_| ())
        }),
        ("group-ruby", |builder, n| {
            builder.group_ruby(range(n), "よみ").map(|_| ())
        }),
        ("jukugo-ruby", |builder, n| {
            builder.jukugo_ruby(range(n), reading(n)).map(|_| ())
        }),
        ("emphasis", |builder, n| {
            builder.emphasis_dots(range(n), '\u{30fb}').map(|_| ())
        }),
        ("tate-chu-yoko", |builder, n| {
            builder.tate_chu_yoko(range(n)).map(|_| ())
        }),
        ("warichu", |builder, n| {
            builder.warichu(range(n)).map(|_| ())
        }),
        ("furawake-2", |builder, n| {
            builder.furawake(range(n), 2, 0.0).map(|_| ())
        }),
        ("furawake-3", |builder, n| {
            builder.furawake(range(n), 3, 0.0).map(|_| ())
        }),
        ("jidori-4", |builder, n| {
            builder.jidori(range(n), 4).map(|_| ())
        }),
        ("reference-mark", |builder, n| {
            builder.reference_mark(range(n), "注").map(|_| ())
        }),
        ("superscript", |builder, n| {
            builder
                .script(range(n), "注", ScriptPosition::Superscript)
                .map(|_| ())
        }),
        ("subscript", |builder, n| {
            builder
                .script(range(n), "注", ScriptPosition::Subscript)
                .map(|_| ())
        }),
        ("formula", |builder, n| {
            builder.formula(range(n)).map(|_| ())
        }),
    ]
}

/// One reading character per base cluster, which mono and jukugo ruby require.
fn reading(clusters: usize) -> String {
    "あいうえお".chars().take(clusters).collect()
}

fn range(clusters: usize) -> std::ops::Range<usize> {
    START..START.saturating_add(clusters.saturating_mul(3))
}

/// The lengths a construct refuses outright, which is an answer rather than a
/// hole in the sweep: a furawake cannot split two columns out of one cluster.
/// Combinations whose geometry holds at one measure and not the other. A
/// construct that only breaks when its line wraps is a different finding from
/// one that breaks outright, so the two are pinned apart.
const MEASURE_DEPENDENT: &[(&str, usize, &str)] = &[];

const REFUSED: &[(&str, usize)] = &[("furawake-2", 1), ("furawake-3", 1), ("furawake-3", 2)];

/// The combinations that do not hold, and nothing else.
///
/// Empty since `docs/adr/0030`. It is kept rather than replaced by a bare
/// `assert!(broken.is_empty())` because the shape is the point: a regression
/// arrives as a row naming the construct, the length and the writing mode, and
/// a deliberate deferral has somewhere to be written down and explained.
const KNOWN_BROKEN: &[(&str, usize, &str)] = &[];

#[test]
fn every_construct_holds_its_geometry_at_every_length() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    let mut broken: Vec<(&str, usize, &str)> = Vec::new();
    let mut refused: Vec<(&str, usize)> = Vec::new();
    let mut measure_dependent: Vec<(&str, usize, &str)> = Vec::new();

    for (name, attach) in constructs() {
        for clusters in 1..=5usize {
            for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
                let mut soundness = Vec::new();
                // Wide enough to hold the paragraph, and narrow enough to wrap
                // it. A construct whose geometry depends on which is its own
                // finding, so the two answers are compared rather than merged.
                for extent in [300.0_f32, 90.0] {
                    let body: String = "一二三四五".chars().take(clusters).collect();
                    let mut builder = DocumentBuilder::new(format!("前と{body}後。"));
                    attach(&mut builder, clusters)
                        .unwrap_or_else(|error| panic!("{name} at {clusters} cluster(s): {error}"));
                    // Some constructs refuse a length outright — two columns
                    // cannot be split out of one cluster. That is an answer,
                    // not a hole, so it is recorded and pinned like the rest.
                    let laid_out = builder.build().and_then(|document| {
                        jlreq::layout_document(
                            &document,
                            &fonts,
                            LayoutOptions::try_new(extent, 16.0)?.with_writing_mode(mode),
                        )
                    });
                    let Ok(layout) = laid_out else {
                        refused.push((name, clusters));
                        continue;
                    };
                    soundness.push(jlreq::verify::inspect(&layout).is_sound());
                }
                let label = match mode {
                    WritingMode::VerticalRl => "VerticalRl",
                    _ => "HorizontalTb",
                };
                if soundness.iter().any(|sound| !sound) {
                    broken.push((name, clusters, label));
                }
                if soundness.windows(2).any(|pair| pair[0] != pair[1]) {
                    measure_dependent.push((name, clusters, label));
                }
            }
        }
    }

    refused.sort_unstable();
    refused.dedup();
    assert_eq!(
        refused, REFUSED,
        "the set of construct lengths the builder refuses moved"
    );

    measure_dependent.sort_unstable();
    assert_eq!(
        measure_dependent, MEASURE_DEPENDENT,
        "the measure decides whether a construct's geometry holds, which is a finding of its own"
    );

    broken.sort_unstable();
    assert_eq!(
        broken, KNOWN_BROKEN,
        "the set of constructs whose geometry does not hold moved"
    );
    Ok(())
}
