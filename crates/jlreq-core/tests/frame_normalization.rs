// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the declared frame does, and what [ADR 0017] says it should do.
//!
//! ADR 0017 is accepted and decides that when a caller declares a frame whose advance
//! already contains a §3.1.2 conditional space, the composer normalizes by *trimming* that
//! amount and reports every unit it took. `Line::trims` does not exist and the trimming does
//! not happen: `Frame` reaches only `spec::narrow_by_frame`, which narrows the class
//! candidates, and `spec::table_one_space_components`, which only ever adds.
//!
//! This file pins that. It asserts the behavior as it is, not as ADR 0017 decides it should
//! be, and it is the fastest way to learn that the decision has been implemented: the moment
//! anyone makes the frame subtract, these assertions fail and point here.
//!
//! Why it is pinned rather than fixed: §3.1.2 covers the five commonest punctuation classes
//! in Japanese, so making the frame subtract moves the advance of `、`, `。`, `「`, `」` and
//! `・` across an entire corpus. The oracle that would catch a mistake in that change is the
//! three-implementation census, and it cannot be run from this repository — the OCaml and
//! Racket engines need toolchains `mise` does not manage, and only a manually dispatched
//! workflow invokes them. `docs/decisions/frame-normalization-unimplemented.md` records the
//! finding and the condition for resuming.
//!
//! [ADR 0017]: ../../../docs/adr/0017-normalized-line-geometry.md

use std::error::Error;

use jlreq_core::{Cluster, Frame, InputError, Paragraph, ShapedText, Size, Style};

/// One em, in the caller's units, for every cluster and both frames.
const EM: i32 = 1_000;

/// A comma and an ideograph: the comma is cl-07, one of the five classes §3.1.2 names.
const SOURCE: &str = "、日";

fn shaped(frame: Frame) -> Result<ShapedText, InputError> {
    let clusters = SOURCE.char_indices().map(|(start, character)| {
        Cluster::new(start..start.saturating_add(character.len_utf8()), EM)
    });
    ShapedText::new(SOURCE, Size::square(EM)?, frame, clusters)
}

fn advances(frame: Frame) -> Result<(i32, Vec<i32>), Box<dyn Error>> {
    let paragraph = Paragraph::builder(shaped(frame)?, 20_000).build()?;
    let layout = jlreq_core::compose(&paragraph, &Style::jlreq_2020())?;
    let line = layout
        .lines()
        .first()
        .ok_or_else(|| std::io::Error::other("one line was expected"))?;
    Ok((
        line.inline_extent(),
        line.clusters()
            .iter()
            .map(jlreq_core::ClusterPlacement::advance)
            .collect(),
    ))
}

/// The declared frame changes nothing about the geometry that reaches the caller.
///
/// If ADR 0017 were implemented, `FullEm` would report the comma at 500 — the caller's em
/// less the half-em §3.1.2 says is already inside it — and `HalfEm` would report 1000 after
/// adding it. Both report 1500 today, because the space is added in both cases.
#[test]
fn every_frame_reports_the_same_advances_today() -> Result<(), Box<dyn Error>> {
    let full = advances(Frame::FullEm)?;
    let half = advances(Frame::HalfEm)?;
    let proportional = advances(Frame::Proportional)?;

    assert_eq!(
        full, half,
        "FullEm and HalfEm already differ; read ADR 0017"
    );
    assert_eq!(
        full, proportional,
        "Proportional already differs; read ADR 0017"
    );
    assert_eq!(
        full,
        (2_500, vec![1_500, 1_000]),
        "the measured geometry moved; if this was the ADR 0017 change, say so and re-run \
         the census before adopting it"
    );
    Ok(())
}

/// The trimming ADR 0017 decides on has no carrier, so nothing could report it even if the
/// composer performed it. `Line` states seven things and a trim list is not among them.
///
/// `xtask/src/conform.rs` already validates a `trims` field — it checks that each entry
/// names §3.1.2 or a Table 1 cell — and no case in `suite.ndjson` carries one, so that
/// validation runs against fixtures only. Both halves are waiting for the same change.
#[test]
fn a_line_still_has_nowhere_to_report_a_trim() -> Result<(), Box<dyn Error>> {
    let paragraph = Paragraph::builder(shaped(Frame::FullEm)?, 20_000).build()?;
    let layout = jlreq_core::compose(&paragraph, &Style::jlreq_2020())?;
    let line = layout
        .lines()
        .first()
        .ok_or_else(|| std::io::Error::other("one line was expected"))?;

    // Everything a line states. A trim list would be an eighth, and adding one is the
    // reporting half of the ADR 0017 change.
    let stated = (
        line.range(),
        line.inline_origin(),
        line.block_origin(),
        line.inline_extent(),
        line.block_extent(),
        line.clusters().len(),
        line.attachments().len(),
    );
    assert_eq!(stated.5, 2);
    assert_eq!(stated.6, 0);
    Ok(())
}
