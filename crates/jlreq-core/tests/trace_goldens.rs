// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Committed decision traces, held byte for byte.
//!
//! The three-implementation census is the project's strongest oracle and it cannot be run
//! here: the OCaml and Racket engines are outside `mise`, so `just census-all` needs a
//! toolchain a contributor may not have, and no push-triggered workflow runs it. What is
//! left is the standing promise that core behavior on existing input does not move —
//! a promise nothing mechanically holds between releases.
//!
//! These goldens hold it, at a resolution the census does not reach. The census compares
//! answers; a trace compares reasoning. A change that reorders the adjustment ladder, or
//! charges a different surcharge, or stops the search one candidate earlier, moves a
//! golden even where the final geometry happens to agree — and geometry agreeing by
//! coincidence is exactly the case a person would otherwise ship.
//!
//! Composition is integer-only, so a golden is identical on Linux, Windows, and macOS;
//! the three-OS matrix holds that as it holds every other layout number.
//!
//! To adopt an intended change, run the suite with `JLREQ_BLESS=1` and read the diff
//! before committing it. A golden that changes without a reason stated in the commit
//! message is the finding, not the noise.

use std::error::Error;
use std::fmt::{self, Write as _};
use std::fs;
use std::path::{Path, PathBuf};

use jlreq_core::trace::{Categories, Trace};
use jlreq_core::{
    Break, Cluster, Construct, Frame, InputError, Paragraph, Ruby, RubyKind, RubyRun, ShapedText,
    Size, Style, WritingMode,
};

/// The environment variable that rewrites the goldens instead of comparing them.
const BLESS: &str = "JLREQ_BLESS";

/// One recorded scenario: a name, a paragraph, and the policy it is set under.
struct Scenario {
    name: &'static str,
    /// What this case is here to hold, printed into the golden so a diff explains itself.
    intent: &'static str,
    paragraph: Paragraph,
    style: Style,
    categories: Categories,
}

fn shaped(source: &str, frame: Frame) -> Result<ShapedText, InputError> {
    let clusters = source.char_indices().map(|(start, character)| {
        Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000)
    });
    ShapedText::new(source, Size::square(1_000)?, frame, clusters)
}

fn every_boundary(source: &str) -> impl Iterator<Item = Break> + '_ {
    source
        .char_indices()
        .skip(1)
        .map(|(offset, _)| Break::allowed(offset))
}

fn plain(source: &str, extent: i32, mode: WritingMode) -> Result<Paragraph, InputError> {
    Paragraph::builder(shaped(source, Frame::FullEm)?, extent)
        .breaks(every_boundary(source))
        .writing_mode(mode)
        .build()
}

/// The recorded corpus.
///
/// Each case is here because it reaches a decision the others do not. Adding a case is
/// cheap and adds coverage; removing one removes a guard, so say why in the commit.
fn corpus() -> Result<Vec<Scenario>, InputError> {
    let ruby_source = "日本語組版";
    let ruby_paragraph = Paragraph::builder(shaped(ruby_source, Frame::FullEm)?, 2_000)
        .constructs([Construct::ruby(Ruby::new(
            RubyKind::Mono,
            0..3,
            shaped("にほ", Frame::FullEm)?,
            [RubyRun::new(0..3, 0..6)],
        )?)])
        .breaks(every_boundary(ruby_source))
        .build()?;

    Ok(vec![
        Scenario {
            name: "horizontal-plain",
            intent: "the ordinary case: an even paragraph broken to fit a narrow measure",
            paragraph: plain("日本語組版処理の要件", 3_000, WritingMode::HorizontalTb)?,
            style: Style::default(),
            categories: Categories::DEFAULT,
        },
        Scenario {
            name: "vertical-plain",
            intent: "the same text set vertically, which must reach the same decisions",
            paragraph: plain("日本語組版処理の要件", 3_000, WritingMode::VerticalRl)?,
            style: Style::default(),
            categories: Categories::DEFAULT,
        },
        Scenario {
            name: "single-line",
            intent: "a measure wide enough that the search never rejects a candidate",
            paragraph: plain("日本語組版", 60_000, WritingMode::HorizontalTb)?,
            style: Style::default(),
            categories: Categories::DEFAULT,
        },
        Scenario {
            name: "punctuation-mojikumi",
            intent: "the five half-width punctuation classes, where Table 1 supplies the space",
            paragraph: plain("日、本。語（版）字", 2_500, WritingMode::HorizontalTb)?,
            style: Style::book_2020(),
            categories: Categories::DEFAULT,
        },
        Scenario {
            name: "kinsoku-refusal",
            intent: "a closing bracket at a boundary, which kinsoku refuses as a break",
            paragraph: plain("あ（い）う（え）お", 2_000, WritingMode::HorizontalTb)?,
            style: Style::default(),
            categories: Categories::DEFAULT.with(Categories::KINSOKU),
        },
        Scenario {
            name: "search-candidates",
            intent: "every pair the search weighed, including the ones it charged and dropped",
            paragraph: plain("日本語組版処理", 2_000, WritingMode::HorizontalTb)?,
            style: Style::default(),
            categories: Categories::ALL,
        },
        Scenario {
            name: "mixed-script",
            intent: "a Japanese/Western boundary, where the expansion opportunity lives",
            paragraph: plain("和文Latin和文", 2_500, WritingMode::HorizontalTb)?,
            style: Style::default(),
            categories: Categories::DEFAULT,
        },
        Scenario {
            name: "hanging-punctuation",
            intent: "a comma at a line end under the book profile, which hangs it past the measure",
            paragraph: plain(
                "あいう、えおか、きくけ、こ",
                3_000,
                WritingMode::HorizontalTb,
            )?,
            style: Style::book_2020(),
            categories: Categories::DEFAULT,
        },
        Scenario {
            name: "mono-ruby",
            intent: "a construct on the line, which stops the indexed fast measurement",
            paragraph: ruby_paragraph,
            style: Style::default(),
            categories: Categories::DEFAULT,
        },
    ])
}

fn goldens_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
}

/// Render one scenario, header and all, exactly as it is stored.
///
/// The header states the outcome as well as the reasoning, so a golden that changes tells
/// the reader immediately whether the answer moved with it or only the route to it.
fn render(scenario: &Scenario) -> Result<String, fmt::Error> {
    let mut trace = Trace::with_categories(scenario.categories);
    let outcome = jlreq_core::compose_traced(&scenario.paragraph, &scenario.style, &mut trace);

    let mut rendered = String::new();
    writeln!(rendered, "# {name}", name = scenario.name)?;
    writeln!(rendered, "# {intent}", intent = scenario.intent)?;
    match outcome {
        Ok(layout) => writeln!(
            rendered,
            "# lines={lines} diagnostics={diagnostics}",
            lines = layout.lines().len(),
            diagnostics = layout.diagnostics().len()
        )?,
        Err(error) => writeln!(
            rendered,
            "# refused resource={resource:?} limit={limit} observed={observed}",
            resource = error.resource(),
            limit = error.limit(),
            observed = error.observed()
        )?,
    }
    write!(rendered, "{trace}")?;
    Ok(rendered)
}

#[test]
fn every_recorded_trace_matches_its_golden() -> Result<(), Box<dyn Error>> {
    let directory = goldens_directory();
    let blessing = std::env::var_os(BLESS).is_some();
    if blessing {
        fs::create_dir_all(&directory)?;
    }

    let mut differences = Vec::new();
    let mut expected_names = Vec::new();
    for scenario in corpus()? {
        let path = directory.join(format!("{}.txt", scenario.name));
        expected_names.push(format!("{}.txt", scenario.name));
        let rendered = render(&scenario)?;

        if blessing {
            fs::write(&path, rendered.as_bytes())?;
            continue;
        }

        match fs::read_to_string(&path) {
            Ok(stored) if stored == rendered => {},
            Ok(stored) => differences.push(format!(
                "{name}: the recorded trace changed\n--- stored\n{stored}--- observed\n{rendered}",
                name = scenario.name,
            )),
            Err(error) => differences.push(format!(
                "{name}: {} could not be read ({error})",
                path.display(),
                name = scenario.name,
            )),
        }
    }

    if !blessing {
        // A golden left behind by a deleted scenario is a stale guard that passes by
        // never running, so the directory is held in both directions.
        let mut stray = Vec::new();
        for entry in fs::read_dir(&directory)? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if !expected_names.contains(&name) {
                stray.push(name);
            }
        }
        stray.sort();
        for name in stray {
            differences.push(format!("{name}: no scenario records this golden any more"));
        }
    }

    assert!(
        differences.is_empty(),
        "{count} recorded trace(s) no longer match. A layout change that moves one is the \
         finding, not the noise: read the diff, and if it is intended re-run with \
         {BLESS}=1 and say why in the commit message.\n\n{report}",
        count = differences.len(),
        report = differences.join("\n"),
    );
    Ok(())
}

/// The corpus must keep reaching the decisions it was assembled to reach.
///
/// Without this, a scenario could quietly stop exercising its family — a narrowed measure,
/// a construct that no longer lowers — and the golden would keep passing while guarding
/// nothing.
#[test]
fn the_corpus_still_reaches_every_family_it_names() -> Result<(), Box<dyn Error>> {
    let mut seen = Vec::new();
    for scenario in corpus()? {
        let mut trace = Trace::with_categories(scenario.categories);
        let _ = jlreq_core::compose_traced(&scenario.paragraph, &scenario.style, &mut trace);
        assert!(
            !trace.is_truncated(),
            "{}: the recorded trace hit its ceiling, so the golden is incomplete",
            scenario.name
        );
        for event in trace.events() {
            if !seen.contains(&event.kind()) {
                seen.push(event.kind());
            }
        }
    }
    seen.sort_unstable();

    assert_eq!(
        seen,
        [
            "expand.residual",
            "expand.site",
            "expand.stage",
            "hang.line-end",
            "line.finished",
            "line.fit",
            "prepare.paragraph",
            "reduce.site",
            "reduce.stage",
            "search.bound-stop",
            "search.candidate",
            "search.chosen",
            "search.refused-candidate",
        ],
        "the corpus no longer reaches the families it was assembled for"
    );
    Ok(())
}

/// Recording must not change the answer, on the same corpus the goldens are cut from.
#[test]
fn recording_does_not_change_the_layout_of_any_recorded_scenario() -> Result<(), Box<dyn Error>> {
    for scenario in corpus()? {
        let mut trace = Trace::with_categories(Categories::ALL);
        let plain_result = jlreq_core::compose(&scenario.paragraph, &scenario.style);
        let traced_result =
            jlreq_core::compose_traced(&scenario.paragraph, &scenario.style, &mut trace);
        assert_eq!(
            plain_result, traced_result,
            "{}: recording changed the answer",
            scenario.name
        );
    }
    Ok(())
}
