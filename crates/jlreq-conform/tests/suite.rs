// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The published suite, run against this workspace.
//!
//! One test per case **file**, not per case, because `docs/design/conformance.md` fixes that
//! unit: the nextest profile reports a test slow at 10 s and terminates it at 60 s with
//! `flaky-result = "fail"`, so a sweep over all 1133 Appendix A keys must be split by
//! section rather than run as one test. The list of sections is written out below and held
//! against the directory by a test of its own, so a case file added without a test is a
//! failure rather than a silence.

use std::error::Error;
use std::path::{Path, PathBuf};

use jlreq_conform::{Kumihan, Suite, load, refusals, run_file};

/// What one of these tests answers with.
///
/// A `Result` rather than a panic, because the workspace forbids `unwrap`, `expect` and
/// `panic!` outside a `#[test]` body and the reading below is done in helpers the tests
/// share.
type Outcome = Result<(), Box<dyn Error>>;

/// One `#[test]` per case file, and the section list the directory is held against.
///
/// Each section carries the census its file must produce: how many of its cases this
/// workspace answers, and how many it does not. Both halves are asserted, and the second is
/// the one that was missing. `measure` used to assert only that nothing disagreed, which an
/// implementation answering `None` to everything satisfies — so the green this suite
/// reported had a floor of zero evidence, and a case that quietly stopped being answerable
/// was a silence rather than a failure. The counts are committed here so that changing one
/// is a reviewable act, in the same way deleting a deferral is.
macro_rules! per_section {
    ($($name:ident => $section:literal [$attempted:literal attempted, $skipped:literal not attempted]),* $(,)?) => {
        $(
            #[test]
            fn $name() -> Outcome {
                measure($section, $attempted, $skipped)
            }
        )*

        /// Every section the suite publishes a file for.
        const SECTIONS: &[&str] = &[$($section),*];
    };
}

per_section! {
    appendix_a_1 => "A.1" [13 attempted, 0 not attempted],
    appendix_a_2 => "A.2" [17 attempted, 0 not attempted],
    appendix_a_3 => "A.3" [10 attempted, 1 not attempted],
    appendix_a_4 => "A.4" [11 attempted, 0 not attempted],
    appendix_a_5 => "A.5" [11 attempted, 2 not attempted],
    appendix_a_6 => "A.6" [7 attempted, 1 not attempted],
    appendix_a_7 => "A.7" [12 attempted, 1 not attempted],
    appendix_a_8 => "A.8" [13 attempted, 0 not attempted],
    appendix_a_9 => "A.9" [11 attempted, 0 not attempted],
    appendix_a_10 => "A.10" [7 attempted, 0 not attempted],
    appendix_a_11 => "A.11" [10 attempted, 0 not attempted],
    appendix_a_12 => "A.12" [8 attempted, 0 not attempted],
    appendix_a_13 => "A.13" [8 attempted, 2 not attempted],
    appendix_a_14 => "A.14" [6 attempted, 0 not attempted],
    appendix_a_15 => "A.15" [14 attempted, 0 not attempted],
    appendix_a_16 => "A.16" [25 attempted, 1 not attempted],
    appendix_a_17 => "A.17" [13 attempted, 0 not attempted],
    appendix_a_18 => "A.18" [14 attempted, 0 not attempted],
    appendix_a_19 => "A.19" [20 attempted, 0 not attempted],
    appendix_a_20 => "A.20" [0 attempted, 11 not attempted],
    appendix_a_21 => "A.21" [3 attempted, 14 not attempted],
    appendix_a_22 => "A.22" [1 attempted, 11 not attempted],
    appendix_a_23 => "A.23" [3 attempted, 5 not attempted],
    appendix_a_24 => "A.24" [13 attempted, 0 not attempted],
    appendix_a_25 => "A.25" [20 attempted, 0 not attempted],
    appendix_a_26 => "A.26" [9 attempted, 0 not attempted],
    appendix_a_27 => "A.27" [22 attempted, 0 not attempted],
    appendix_a_28 => "A.28" [2 attempted, 12 not attempted],
    appendix_a_29 => "A.29" [1 attempted, 15 not attempted],
    appendix_a_30 => "A.30" [5 attempted, 6 not attempted],
}

/// The published cases directory.
fn cases() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("cases")
}

/// The whole suite, read once per test.
fn suite() -> Result<Suite, Box<dyn Error>> {
    Ok(load(&cases())?)
}

/// Run one file's cases against this workspace and report every disagreement.
///
/// Three assertions, not one. The disagreements are the finding everyone expects; the two
/// counts are what stop an empty answer from reading as agreement. A case that becomes
/// unanswerable moves from `attempted` to `skipped` without disagreeing with anything, and a
/// layer that starts answering moves it the other way — either is a change in what this
/// suite proves, and both are stated in numbers rather than being a silence.
fn measure(section: &str, attempted: usize, skipped: usize) -> Outcome {
    let suite = suite()?;
    let file = suite
        .file(section)
        .ok_or_else(|| format!("the suite publishes no file for §{section}"))?;
    let report = run_file(file, &Kumihan::default());
    println!("§{section}: {census}", census = report.census());
    assert!(
        report.disagreed.is_empty(),
        "§{section}: {count} case(s) disagree with this workspace:\n\n{findings}",
        count = report.disagreed.len(),
        findings = report
            .disagreed
            .iter()
            .map(jlreq_conform::Disagreement::message)
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    assert_eq!(
        (report.attempted, report.skipped),
        (attempted, skipped),
        "§{section}: this file's census has moved. The counts in this test are what say how \
         much of the section this workspace answers, and an empty `disagreed` is satisfied \
         by answering nothing at all"
    );
    Ok(())
}

#[test]
fn every_case_file_has_a_test_of_its_own() -> Outcome {
    let suite = suite()?;
    let mut found: Vec<&str> = suite
        .files()
        .iter()
        .map(jlreq_conform::CaseFile::section)
        .collect();
    let mut sections: Vec<&str> = SECTIONS.to_vec();
    sections.sort_unstable();
    found.sort_unstable();
    assert_eq!(
        found, sections,
        "the section list in this file and the published cases directory have parted; a case \
         file with no test of its own is a file nothing runs"
    );
    Ok(())
}

/// How many permitted entries `Policy::JLREQ` cannot select today.
///
/// Every one of them names a question the generated policy space does not have yet:
/// `spec/derived/questions.tsv` inventories twenty-one and stage 2 of the derivation, which
/// turns them into `Question` constants, has not run. So the entries are published readings
/// that nothing evaluates, and the cases carrying them assert what their `{}` entry says.
///
/// The number is committed rather than reported, because a run in which it changes is a run
/// in which the suite proves something different: it falls as the policy space is generated
/// and every reading becomes selectable, and it rises whenever a case gains a reading this
/// workspace cannot yet be measured against. Either is a reviewable act.
const UNSELECTABLE: usize = 170;

#[test]
fn every_published_reading_this_policy_cannot_select_is_counted() -> Outcome {
    let report = jlreq_conform::run(&suite()?, &Kumihan::default());
    println!("suite: {census}", census = report.census());
    assert_eq!(
        report.unselectable, UNSELECTABLE,
        "a permitted entry naming a question the declared policy does not have applies to \
         nothing, so it is neither matched nor reported as a difference. That is the right \
         behavior and the wrong silence, and this is the number that ends it"
    );
    Ok(())
}

#[test]
fn every_published_input_is_one_this_workspace_would_build() -> Outcome {
    let found = refusals(&suite()?);
    assert!(
        found.is_empty(),
        "`conform --check` and `Text::new` are two implementations of ADR 0018's invariants, \
         and they have parted over {count} case(s):\n{findings}",
        count = found.len(),
        findings = found.join("\n")
    );
    Ok(())
}
