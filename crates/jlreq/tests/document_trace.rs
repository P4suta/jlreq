// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Committed document traces, held byte for byte, and the proof that recording one changes
//! nothing.
//!
//! `crates/jlreq-core/tests/trace_goldens.rs` does this for composition. The facade needs
//! its own because it has two failure modes the core does not: [`LayoutEngine`] keeps font
//! and shaper caches *between* calls, and one call's [`CallState`] accumulates across every
//! paragraph. Either could make a traced call diverge from an untraced one in a way a
//! single-paragraph comparison would never reach. So the equality tests here run whole
//! documents, and then run the traced engine again to show it came back unchanged.
//!
//! Every dependency that can move a number here is pinned to an exact version in
//! `crates/jlreq/Cargo.toml`, so a golden that moves is a change in this repository.
//!
//! One thing to read past rather than chase: the fixture faces are subsets, so ordinary
//! Japanese text records `face.fallback` in these goldens. That is the fixture speaking,
//! not a defect — the selection logic is doing exactly what it should with a face that
//! genuinely lacks the codepoint, and `face-fallback.txt` is the case assembled to show
//! selection succeeding as well as failing.
//!
//! To adopt an intended change, run the suite with `JLREQ_BLESS=1` and read the diff
//! before committing it. A golden that changes without a reason stated in the commit
//! message is the finding, not the noise.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use jlreq::trace::{Categories, DocumentTrace};
use jlreq::{
    BaseDirection, Document, DocumentBuilder, FontLibrary, FontStyle, LayoutEngine, LayoutError,
    LayoutOptions, WritingMode,
};

/// The environment variable that rewrites the goldens instead of comparing them.
const BLESS: &str = "JLREQ_BLESS";

/// One recorded scenario: a name, a document, and the options it is laid out under.
struct Scenario {
    name: &'static str,
    /// What this case is here to hold, printed into the golden so a diff explains itself.
    intent: &'static str,
    document: Document,
    options: LayoutOptions,
    categories: Categories,
    core_categories: jlreq::core::trace::Categories,
}

fn bytes(data: &'static [u8]) -> Arc<[u8]> {
    Arc::from(data)
}

/// The same four faces the acceptance suite registers, in the same order, so a face
/// ordinal in a golden means the same thing there as it does in `high_level.rs`.
fn fixture_fonts() -> Result<FontLibrary, LayoutError> {
    let mut fonts = FontLibrary::new();
    fonts.register_face(
        bytes(font_test_data::NOTO_SANS_JP_CFF),
        0,
        "Noto Sans JP",
        FontStyle::default(),
    )?;
    fonts.register_face(
        bytes(rwml_fonts::noto_sans_arabic_subset()),
        0,
        "Vazirmatn",
        FontStyle::default(),
    )?;
    fonts.register_face(
        bytes(font_test_data::NOTO_COLOR_EMOJI_FLAGS),
        0,
        "Noto Color Emoji",
        FontStyle::default(),
    )?;
    fonts.register_face(
        bytes(font_test_data::TINOS_SUBSET),
        0,
        "Tinos",
        FontStyle::default(),
    )?;
    Ok(fonts)
}

fn corpus() -> Result<Vec<Scenario>, Box<dyn Error>> {
    let default_core = jlreq::core::trace::Categories::DEFAULT;
    Ok(vec![
        Scenario {
            name: "horizontal-plain",
            intent: "one ordinary paragraph: segmentation, itemization, faces, and the \
                     core's own reasoning in one stream",
            document: DocumentBuilder::new("日本語の組版、その理由。").build()?,
            options: LayoutOptions::try_new(120.0, 16.0)?,
            categories: Categories::ALL,
            core_categories: default_core,
        },
        Scenario {
            name: "vertical-plain",
            intent: "the same text set vertically: the run direction and the core's \
                     placement transforms both change",
            document: DocumentBuilder::new("日本語の組版、その理由。").build()?,
            options: LayoutOptions::try_new(120.0, 16.0)?
                .with_writing_mode(WritingMode::VerticalRl),
            categories: Categories::ALL,
            core_categories: default_core,
        },
        Scenario {
            name: "paragraphs",
            intent: "three paragraphs, the middle one blank: each states its own resolved \
                     style, and a blank one never reaches the composer",
            document: DocumentBuilder::new("最初の段落。\n\n最後の段落。").build()?,
            options: LayoutOptions::try_new(120.0, 16.0)?,
            categories: Categories::ALL,
            core_categories: jlreq::core::trace::Categories::NONE,
        },
        Scenario {
            name: "face-fallback",
            intent: "why each glyph came from the face it did: Japanese, Latin, Arabic, \
                     and an emoji no primary face covers",
            document: DocumentBuilder::new("日本Aب🇪🇨").build()?,
            options: LayoutOptions::try_new(200.0, 16.0)?,
            categories: Categories::FACES
                .with(Categories::RUNS)
                .with(Categories::ITEMIZE),
            core_categories: jlreq::core::trace::Categories::NONE,
        },
        Scenario {
            name: "bidi-run",
            intent: "a declared right-to-left base direction: the itemization reports the \
                     resolved paragraph level and that the levels are mixed",
            document: DocumentBuilder::new("بالعربية and Latin").build()?,
            options: LayoutOptions::try_new(200.0, 16.0)?
                .with_base_direction(BaseDirection::RightToLeft),
            categories: Categories::ITEMIZE.with(Categories::RUNS),
            core_categories: jlreq::core::trace::Categories::NONE,
        },
        Scenario {
            name: "ruby",
            intent: "an annotation is itemized and shaped in its own right, and its trace \
                     sites name the construct rather than a string the document lacks",
            document: ruby_document()?,
            options: LayoutOptions::try_new(160.0, 16.0)?,
            categories: Categories::ALL,
            core_categories: default_core,
        },
        Scenario {
            name: "breaks",
            intent: "the opportunities that actually reached the search, counted from the \
                     list handed over rather than from the sources it merged",
            document: break_document()?,
            options: LayoutOptions::try_new(120.0, 16.0)?,
            categories: Categories::BREAKS.with(Categories::PARAGRAPHS),
            core_categories: jlreq::core::trace::Categories::SEARCH,
        },
        Scenario {
            name: "tight-measure",
            intent: "a measure barely wider than two em: the adjustment ladder runs, and \
                     the facade's framing says which document bytes each rung belongs to",
            document: DocumentBuilder::new("日本語、その組版。").build()?,
            options: LayoutOptions::try_new(40.0, 16.0)?,
            categories: Categories::PARAGRAPHS.with(Categories::BREAKS),
            core_categories: default_core,
        },
        Scenario {
            name: "shared-space",
            intent: "the two places a cell's charged advance is not the step to its \
                     neighbour: a Japanese-Latin boundary bills its quarter em to both \
                     sides, and a tate-chu-yoko run's halves share one coordinate. \
                     Deriving positions from advances alone was wrong at both, and no \
                     golden reached either until this one",
            document: shared_space_document()?,
            // Tight enough that the adjustment ladder must give the line back its
            // remainder, which is what makes a boundary space shrink below the
            // advance it was charged to.
            options: LayoutOptions::try_new(165.0, 16.0)?
                .with_writing_mode(WritingMode::VerticalRl),
            categories: Categories::PLACEMENT.with(Categories::PARAGRAPHS),
            core_categories: default_core.with(jlreq::core::trace::Categories::PLACE_CLUSTERS),
        },
        Scenario {
            name: "constructs",
            intent: "stacked structures and every placed cluster, so a document trace can \
                     answer where one glyph ended up and under which local transform",
            document: construct_document()?,
            options: LayoutOptions::try_new(160.0, 16.0)?
                .with_writing_mode(WritingMode::VerticalRl),
            categories: Categories::PARAGRAPHS,
            core_categories: default_core.with(jlreq::core::trace::Categories::PLACE_CLUSTERS),
        },
    ])
}

fn ruby_document() -> Result<Document, Box<dyn Error>> {
    let mut builder = DocumentBuilder::new("日本語に振り仮名。");
    builder.group_ruby(0..9, "にほんご")?;
    Ok(builder.build()?)
}

fn construct_document() -> Result<Document, Box<dyn Error>> {
    let text = "注釈12と割注。";
    let mut builder = DocumentBuilder::new(text);
    builder.tate_chu_yoko(6..8)?;
    builder.warichu(11..17)?;
    Ok(builder.build()?)
}

/// Both places where a cell's charged advance is not the step to its neighbour.
///
/// `語A` and `A語` each place one quarter em that §3.1 bills to the boundary, so
/// the following cluster begins inside the preceding advance; the two digits of
/// the tate-chu-yoko run share one inline coordinate and differ only in block.
fn shared_space_document() -> Result<Document, Box<dyn Error>> {
    let text = "日本語Aと縦中横12。";
    let mut builder = DocumentBuilder::new(text);
    builder.tate_chu_yoko(22..24)?;
    Ok(builder.build()?)
}

fn break_document() -> Result<Document, Box<dyn Error>> {
    let text = "日本語テキストの改行機会";
    let mut builder = DocumentBuilder::new(text);
    builder.mandatory_break(15)?;
    builder.discretionary_break(21)?;
    Ok(builder.build()?)
}

fn record(scenario: &Scenario) -> Result<String, Box<dyn Error>> {
    let fonts = fixture_fonts()?;
    let mut engine = LayoutEngine::new();
    let mut trace = DocumentTrace::with_categories(scenario.categories, scenario.core_categories);
    let laid_out = engine.layout_document_traced(
        &scenario.document,
        &fonts,
        scenario.options.clone(),
        &mut trace,
    );
    // A refusal is a recordable outcome; what matters is that the reasoning survived it.
    let outcome = match laid_out {
        Ok(layout) => format!("lines={}", layout.lines().len()),
        Err(error) => format!("refused={error}"),
    };
    Ok(format!(
        "# {name}\n# {intent}\n# {outcome}\n{trace}",
        name = scenario.name,
        intent = scenario.intent,
    ))
}

fn goldens() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
}

#[test]
fn every_recorded_trace_matches_its_golden() -> Result<(), Box<dyn Error>> {
    let directory = goldens();
    fs::create_dir_all(&directory)?;
    let bless = std::env::var_os(BLESS).is_some();
    let mut expected = BTreeSet::new();
    for scenario in corpus()? {
        let path = directory.join(format!("{name}.txt", name = scenario.name));
        expected.insert(path.clone());
        let recorded = record(&scenario)?;
        if bless {
            fs::write(&path, recorded.as_bytes())?;
            continue;
        }
        let golden = fs::read_to_string(&path).map_err(|error| {
            std::io::Error::other(format!(
                "{name}: {error}; run with {BLESS}=1 to write it",
                name = scenario.name
            ))
        })?;
        assert_eq!(
            golden.replace("\r\n", "\n"),
            recorded,
            "{name} drifted from its golden; read the diff, then re-bless it deliberately",
            name = scenario.name
        );
    }
    for entry in fs::read_dir(&directory)? {
        let path = entry?.path();
        assert!(
            expected.contains(&path),
            "{path:?} belongs to no scenario; delete it or add the case back"
        );
    }
    Ok(())
}

#[test]
fn the_corpus_still_reaches_every_family_it_names() -> Result<(), Box<dyn Error>> {
    let mut kinds = BTreeSet::new();
    for scenario in corpus()? {
        let fonts = fixture_fonts()?;
        let mut engine = LayoutEngine::new();
        let mut trace =
            DocumentTrace::with_categories(scenario.categories, scenario.core_categories);
        let _ = engine.layout_document_traced(
            &scenario.document,
            &fonts,
            scenario.options.clone(),
            &mut trace,
        );
        kinds.extend(trace.events().iter().map(|event| event.kind().to_owned()));
    }
    let reached: Vec<_> = kinds.iter().map(String::as_str).collect();
    // Every facade kind, plus the core kinds these documents happen to reach. The core
    // vocabulary is exercised exhaustively by `jlreq-core`'s own goldens; what this list
    // holds is that absorption keeps working and that no facade kind quietly stops firing.
    assert_eq!(
        reached,
        [
            "draw.cell",
            "draw.line",
            "expand.residual",
            "expand.site",
            "expand.stage",
            "face.chosen",
            "face.fallback",
            "line.finished",
            "line.fit",
            "para.segment",
            "place.cluster",
            "prepare.paragraph",
            "reduce.site",
            "reduce.stage",
            "search.chosen",
            "space.boundary",
            "tcy.group",
            "text.breaks",
            "text.itemized",
            "text.run",
            "text.segmented",
            "warichu.block",
        ],
        "the corpus stopped exercising a family, or started exercising a new one"
    );
    Ok(())
}

/// The corpus is here to make the *trace* branch, which makes it the widest set
/// of composed documents in the crate — bidi, emoji fallback, warichu, stacked
/// structures, a blank paragraph, a measure barely wider than two em. Asking
/// `jlreq::verify` about each one costs nothing and gives every geometric
/// statement far more witnesses than `tests/geometry.rs` can assemble on its
/// own. `jlreq-core`'s own goldens do exactly this with the core's checker.
#[test]
fn every_recorded_layout_is_geometrically_sound() -> Result<(), Box<dyn Error>> {
    // One known defect, stated rather than skipped. The facade never reduces a
    // warichu to the smaller size §3.4 sets it at, so its two lanes are placed
    // at the paragraph's own em inside the one em the line reserved, so each
    // overhangs its line by half an em along the block axis — onto the line
    // beside it. `crates/jlreq/tests/construct_size.rs` pins the geometry and
    // records why choosing the size is deferred. When it is chosen, this
    // expectation is what tells whoever chose it to come here.
    //
    // Counted per (scenario, kind) and compared in sorted order rather than as
    // a positional list, so that adding a scenario to the corpus or reordering
    // the statements inside `inspect` cannot fail this test for a reason that
    // is not geometric. A genuinely new fault appears as its own row, naming
    // the scenario it came from.
    let known: &[(&str, &str, usize)] = &[("constructs", "cell-escapes-its-line", 2)];

    let mut counted: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for scenario in corpus()? {
        let fonts = fixture_fonts()?;
        let mut engine = LayoutEngine::new();
        let layout =
            engine.layout_document(&scenario.document, &fonts, scenario.options.clone())?;
        for fault in jlreq::verify::inspect(&layout).faults() {
            *counted.entry((scenario.name, fault.kind())).or_default() += 1;
        }
    }
    let observed: Vec<(&str, &str, usize)> = counted
        .into_iter()
        .map(|((scenario, kind), count)| (scenario, kind, count))
        .collect();
    assert_eq!(observed, known, "the set of known geometric defects moved");
    Ok(())
}

#[test]
fn recording_changes_neither_the_layout_nor_the_engine() -> Result<(), Box<dyn Error>> {
    for scenario in corpus()? {
        let fonts = fixture_fonts()?;

        let mut plain = LayoutEngine::new();
        let untraced = plain.layout_document(&scenario.document, &fonts, scenario.options.clone());

        let mut recorded = LayoutEngine::new();
        let mut trace = DocumentTrace::new();
        let traced = recorded.layout_document_traced(
            &scenario.document,
            &fonts,
            scenario.options.clone(),
            &mut trace,
        );

        assert_eq!(
            outcome(&untraced),
            outcome(&traced),
            "{name}: recording moved the outcome",
            name = scenario.name
        );

        // The engine keeps caches between calls, and one call's state accumulates across
        // paragraphs. Neither may carry anything the trace put there.
        let after = recorded.layout_document(&scenario.document, &fonts, scenario.options.clone());
        assert_eq!(
            outcome(&untraced),
            outcome(&after),
            "{name}: tracing left the engine in a different state",
            name = scenario.name
        );
    }
    Ok(())
}

/// Both halves of the result in one comparable value.
///
/// A layout compares exactly; a refusal compares by the sentence a caller would read.
/// Pairing them means a scenario that stops refusing, or starts, fails on the same
/// assertion as one whose geometry moved rather than slipping through a match arm.
fn outcome(
    result: &Result<jlreq::TextLayout, LayoutError>,
) -> (Option<&jlreq::TextLayout>, String) {
    match result {
        Ok(layout) => (Some(layout), String::new()),
        Err(error) => (None, error.to_string()),
    }
}

#[test]
fn a_narrowed_trace_records_only_what_it_names() -> Result<(), Box<dyn Error>> {
    let fonts = fixture_fonts()?;
    let document = DocumentBuilder::new("日本語の組版、その理由。").build()?;
    let options = LayoutOptions::try_new(120.0, 16.0)?;

    let mut engine = LayoutEngine::new();
    let mut everything = DocumentTrace::new();
    let _ = engine.layout_document_traced(&document, &fonts, options.clone(), &mut everything)?;

    let mut engine = LayoutEngine::new();
    let mut faces_only =
        DocumentTrace::with_categories(Categories::FACES, jlreq::core::trace::Categories::NONE);
    let _ = engine.layout_document_traced(&document, &fonts, options, &mut faces_only)?;

    assert!(everything.events().len() > faces_only.events().len());
    assert!(!faces_only.events().is_empty());
    assert!(
        faces_only
            .events()
            .iter()
            .all(|event| event.category() == Categories::FACES),
        "a narrowed trace recorded a family it did not name"
    );
    Ok(())
}

#[test]
fn the_ceiling_bounds_the_whole_document_rather_than_one_paragraph() -> Result<(), Box<dyn Error>> {
    let fonts = fixture_fonts()?;
    let document = DocumentBuilder::new("最初の段落。\n二つ目の段落。\n三つ目の段落。").build()?;
    let options = LayoutOptions::try_new(120.0, 16.0)?;
    let mut engine = LayoutEngine::new();
    let mut trace = DocumentTrace::new();
    trace.set_max_events(5);
    let layout = engine.layout_document_traced(&document, &fonts, options, &mut trace)?;

    assert_eq!(trace.events().len(), 5);
    assert!(trace.is_truncated());
    assert!(
        !layout.lines().is_empty(),
        "a full trace must never turn a composable document into a refusal"
    );
    Ok(())
}
