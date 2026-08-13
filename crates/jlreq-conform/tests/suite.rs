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
use jlreq_spec::{Choice, Policy, Question};

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
    section_3_5_1 => "3.5.1" [1 attempted, 0 not attempted],
    section_3_5_2 => "3.5.2" [1 attempted, 0 not attempted],
    section_3_5_3 => "3.5.3" [1 attempted, 0 not attempted],
    section_3_5_4 => "3.5.4" [3 attempted, 0 not attempted],
    section_3_6_1 => "3.6.1" [1 attempted, 0 not attempted],
    section_3_6_2 => "3.6.2" [4 attempted, 0 not attempted],
    section_3_6_3 => "3.6.3" [4 attempted, 0 not attempted],
    section_3_7_3 => "3.7.3" [1 attempted, 0 not attempted],
    section_3_8_1 => "3.8.1" [1 attempted, 0 not attempted],
    section_3_8_2 => "3.8.2" [1 attempted, 0 not attempted],
    section_3_8_3 => "3.8.3" [1 attempted, 0 not attempted],
    section_3_8_4 => "3.8.4" [2 attempted, 0 not attempted],
    section_3_1_12 => "3.1.12" [3 attempted, 0 not attempted],
    appendix_a_1 => "A.1" [13 attempted, 0 not attempted],
    appendix_a_2 => "A.2" [17 attempted, 0 not attempted],
    appendix_a_3 => "A.3" [11 attempted, 0 not attempted],
    appendix_a_4 => "A.4" [11 attempted, 0 not attempted],
    appendix_a_5 => "A.5" [13 attempted, 0 not attempted],
    appendix_a_6 => "A.6" [8 attempted, 0 not attempted],
    appendix_a_7 => "A.7" [13 attempted, 0 not attempted],
    appendix_a_8 => "A.8" [13 attempted, 0 not attempted],
    appendix_a_9 => "A.9" [11 attempted, 0 not attempted],
    appendix_a_10 => "A.10" [7 attempted, 0 not attempted],
    appendix_a_11 => "A.11" [10 attempted, 0 not attempted],
    appendix_a_12 => "A.12" [8 attempted, 0 not attempted],
    appendix_a_13 => "A.13" [10 attempted, 0 not attempted],
    appendix_a_14 => "A.14" [6 attempted, 0 not attempted],
    appendix_a_15 => "A.15" [14 attempted, 0 not attempted],
    appendix_a_16 => "A.16" [25 attempted, 1 not attempted],
    appendix_a_17 => "A.17" [13 attempted, 0 not attempted],
    appendix_a_18 => "A.18" [14 attempted, 0 not attempted],
    appendix_a_19 => "A.19" [20 attempted, 0 not attempted],
    appendix_a_20 => "A.20" [0 attempted, 11 not attempted],
    appendix_a_21 => "A.21" [3 attempted, 14 not attempted],
    appendix_a_22 => "A.22" [2 attempted, 11 not attempted],
    appendix_a_23 => "A.23" [3 attempted, 5 not attempted],
    appendix_a_24 => "A.24" [13 attempted, 0 not attempted],
    appendix_a_25 => "A.25" [20 attempted, 0 not attempted],
    appendix_a_26 => "A.26" [9 attempted, 0 not attempted],
    appendix_a_27 => "A.27" [22 attempted, 0 not attempted],
    appendix_a_28 => "A.28" [2 attempted, 12 not attempted],
    appendix_a_29 => "A.29" [1 attempted, 15 not attempted],
    appendix_a_30 => "A.30" [5 attempted, 6 not attempted],
    appendix_b => "B" [1 attempted, 0 not attempted],
    appendix_b_2 => "B.2" [7 attempted, 0 not attempted],
    appendix_c => "C" [1 attempted, 0 not attempted],
    appendix_c_2 => "C.2" [16 attempted, 0 not attempted],
    appendix_d => "D" [1 attempted, 0 not attempted],
    appendix_d_1 => "D.1" [1 attempted, 0 not attempted],
    appendix_d_2 => "D.2" [4 attempted, 0 not attempted],
    appendix_e => "E" [1 attempted, 0 not attempted],
    appendix_e_2 => "E.2" [7 attempted, 0 not attempted],
    section_3_1_9 => "3.1.9" [1 attempted, 0 not attempted],
    section_3_1_4 => "3.1.4" [3 attempted, 0 not attempted],
    section_3_2_2 => "3.2.2" [3 attempted, 0 not attempted],
    section_3_1_6 => "3.1.6" [14 attempted, 0 not attempted],
    section_3_1_5 => "3.1.5" [3 attempted, 0 not attempted],
    section_3_3_5 => "3.3.5" [5 attempted, 0 not attempted],
    section_3_3_6 => "3.3.6" [4 attempted, 0 not attempted],
    section_3_3_7 => "3.3.7" [3 attempted, 0 not attempted],
    section_3_3_8 => "3.3.8" [2 attempted, 0 not attempted],
}

/// The published cases directory.
fn cases() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("cases")
}

/// The whole suite, read once per test.
fn suite() -> Result<Suite, Box<dyn Error>> {
    Ok(load(&cases())?)
}

/// Run one file's cases against `Kumihan::default()` and report every disagreement.
///
/// The overwhelming majority of these tests measure the workspace's own default reading, so
/// this stays the name every `per_section!` row calls; `measure_against` (below) is the same
/// three assertions parameterized over which implementation answers, factored out once a
/// second construction — `Kumihan::new` under a declared `ruby.alignment: katatsuki` — needed
/// the identical census-and-disagreement check over the same file without a second copy of
/// the three assertions to keep in sync.
fn measure(section: &str, attempted: usize, skipped: usize) -> Outcome {
    measure_against(section, attempted, skipped, Kumihan::default())
}

/// Three assertions, not one. The disagreements are the finding everyone expects; the two
/// counts are what stop an empty answer from reading as agreement. A case that becomes
/// unanswerable moves from `attempted` to `skipped` without disagreeing with anything, and a
/// layer that starts answering moves it the other way — either is a change in what this
/// suite proves, and both are stated in numbers rather than being a silence.
///
/// `implementation` is by value: `Kumihan` derives `Copy` and holds nothing but a `Policy`,
/// itself a fixed-size byte array no larger than the generated question count, well inside
/// `clippy.toml`'s own `trivial-copy-size-limit`, so passing a reference here would only add
/// an indirection this call site does not need.
fn measure_against(
    section: &str,
    attempted: usize,
    skipped: usize,
    implementation: Kumihan,
) -> Outcome {
    let suite = suite()?;
    let file = suite
        .file(section)
        .ok_or_else(|| format!("the suite publishes no file for §{section}"))?;
    let report = run_file(file, &implementation);
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

/// One permitted answer to a question, by its stable name — `crates/jlreq-spacing/src/
/// evaluate.rs`'s own test-module `choice` helper, the established idiom reused here rather
/// than duplicated with different behavior.
fn choice(question: Question, name: &str) -> Result<Choice, Box<dyn Error>> {
    question
        .permits()
        .iter()
        .find(|choice| choice.name() == name)
        .copied()
        .ok_or_else(|| format!("`{name}` is not one of {question:?}'s answers").into())
}

#[test]
fn section_3_3_5_is_also_measured_under_katatsuki() -> Outcome {
    // `Kumihan::default()` (above) declares `Policy::JLREQ`, whose own `ruby.alignment` is
    // nakatsuki, so the runner's own selection rule (`crates/jlreq-conform/src/run.rs`'s own
    // module doc) never picks a `permitted` entry naming `ruby.alignment: katatsuki` for it —
    // every katatsuki reading in `3.3.5.json` is published and checked by `conform --check`
    // but never *attempted* by that run. This test is the second implementation those
    // entries are a statement to, made real: a `Kumihan` declaring `ruby.alignment:
    // katatsuki` selects the katatsuki entry instead, over the identical case file, so the
    // same three `place` cases and two `lower` cases are attempted and agreed a second time,
    // under the other reading. Decline conditions read no policy at all — `Compose::place`'s
    // and `Compose::lower`'s own doc comments in `crates/jlreq-conform/src/kumihan.rs` name
    // exactly what each declines on, and `ruby.alignment` is not among them — so the census
    // below is identical to `section_3_3_5`'s own, not merely close to it: what moves between
    // the two runs is *which* permitted entry is selected, never how many cases this
    // workspace attempts or skips.
    let katatsuki = choice(Question::RUBY_ALIGNMENT, "katatsuki")?;
    let policy = Policy::JLREQ
        .with(katatsuki)
        .map_err(|conflict| format!("{conflict:?}"))?;
    measure_against("3.3.5", 5, 0, Kumihan::new(policy))
}

#[test]
fn section_3_3_6_is_also_measured_under_flush() -> Outcome {
    // `Kumihan::default()` (above) declares `Policy::JLREQ`, whose own `ruby.
    // group_distribution` is `jis`, so the runner's own selection rule (`crates/jlreq-conform/
    // src/run.rs`'s own module doc) never picks a `permitted` entry naming `ruby.
    // group_distribution: flush` for it — every `flush` reading in `3.3.6.json` is published
    // and checked by `conform --check` but never *attempted* by that run. This test is the
    // second implementation those entries are a statement to, made real, the identical shape
    // `section_3_3_5_is_also_measured_under_katatsuki` already gives §3.3.5's own katatsuki
    // entries: a `Kumihan` declaring `ruby.group_distribution: flush` selects the `flush`
    // entry instead, over the identical case file, so `3.3.6/group-ruby-placement/jis-versus-
    // flush-distribution` and `3.3.6/group-ruby-placement/single-ruby-character-jis-vs-flush`
    // are attempted and agreed a second time, under the other reading — without this second
    // run, `flush`'s own arithmetic is a statement published for another implementation to
    // check, never a checked agreement for this one. `place()`'s own decline conditions read
    // no policy at all — §3.3.5(c)'s katatsuki-with-overflow choice and §3.3.6 paragraph 3's
    // ruby-longer-than-base half are both extent comparisons made before either alignment
    // question is ever read (`crates/jlreq-conform/src/kumihan.rs`'s own `Compose::place` doc)
    // — so the census below is identical to `section_3_3_6`'s own, not merely close to it:
    // what moves between the two runs is *which* permitted entry is selected, never how many
    // cases this workspace attempts or skips.
    let flush = choice(Question::GROUP_RUBY_DISTRIBUTION, "flush")?;
    let policy = Policy::JLREQ
        .with(flush)
        .map_err(|conflict| format!("{conflict:?}"))?;
    measure_against("3.3.6", 4, 0, Kumihan::new(policy))
}

#[test]
fn section_3_3_7_is_also_measured_under_phonetic() -> Outcome {
    // `Kumihan::default()` (above) declares `Policy::JLREQ`, whose own `ruby.jukugo_layout`
    // is `group`, so the runner's own selection rule (`crates/jlreq-conform/src/run.rs`'s own
    // module doc) never picks a `permitted` entry naming `ruby.jukugo_layout: phonetic` for
    // it — the `phonetic` reading in `3.3.7/jukugo-ruby-placement/paragraph-two-whole-
    // compound-attachment` is published and checked by `conform --check` but never
    // *attempted* by that run. This test is the second implementation that entry is a
    // statement to, made real, the identical shape `section_3_3_5_is_also_measured_under_
    // katatsuki` and `section_3_3_6_is_also_measured_under_flush` already give their own
    // sibling sections: a `Kumihan` declaring `ruby.jukugo_layout: phonetic` selects the
    // decline instead, over the identical case file, so the compound is attempted and agreed
    // a second time, under the other reading, without this second run being a statement
    // published for another implementation to check and never checked for this one.
    // `place_jukugo`'s own paragraph-1 branch never reads `Question::JUKUGO_RUBY_LAYOUT` at
    // all — only `place_jukugo_compound`'s own paragraph-2 branch does — so the paragraph-1
    // case's own two entries (naming `{}` and `ruby.alignment: katatsuki`, neither of which
    // names `ruby.jukugo_layout`) select their identical `{}` reading under this policy too,
    // and the alignment-discouraged `lower` case is unaffected for the identical reason: the
    // census below is identical to `section_3_3_7`'s own, not merely close to it, because what
    // moves between the two runs is *which* permitted entry the paragraph-2 case selects,
    // never how many cases this workspace attempts or skips.
    let phonetic = choice(Question::JUKUGO_RUBY_LAYOUT, "phonetic")?;
    let policy = Policy::JLREQ
        .with(phonetic)
        .map_err(|conflict| format!("{conflict:?}"))?;
    measure_against("3.3.7", 3, 0, Kumihan::new(policy))
}

#[test]
fn section_3_3_7_is_also_measured_under_flush() -> Outcome {
    // A cousin of `section_3_3_6_is_also_measured_under_flush`, not quite its identical
    // shape: `Policy::JLREQ`'s own default `ruby.group_distribution` is `jis`, so a `Kumihan`
    // declaring `ruby.group_distribution: flush` is what selects `3.3.7/jukugo-ruby-
    // placement/paragraph-two-whole-compound-attachment`'s own third entry (naming both
    // `ruby.jukugo_layout: group` and `ruby.group_distribution: flush`) instead of its first
    // — but that entry asserts the *identical* jis geometry as the first, by
    // `decision:jukugo-group-layout-distribution`'s own forcing, so this run is not
    // self-evidencing the way `section_3_3_7_is_also_measured_under_phonetic` is: the first
    // entry, naming nothing, would already assert those same numbers under this policy even
    // if the third entry did not exist, so a green run here does not by itself prove the
    // third entry was the one selected. What this run does check is the number itself, under
    // a policy that would move an *ordinary* group-ruby run's own numbers at this identical
    // surplus (`3.3.6/group-ruby-placement/jis-versus-flush-distribution`'s own sibling case)
    // — so a regression that let a jukugo compound start reading `Question::GROUP_RUBY_
    // DISTRIBUTION` after all would still be caught here, by disagreement with the first
    // entry's own numbers, which stays selected regardless. The paragraph-1 case is
    // unaffected — its own two entries name no `ruby.group_distribution` at all — and neither
    // is the `lower` case, so the census below is identical to `section_3_3_7`'s own, not
    // merely close to it.
    let flush = choice(Question::GROUP_RUBY_DISTRIBUTION, "flush")?;
    let policy = Policy::JLREQ
        .with(flush)
        .map_err(|conflict| format!("{conflict:?}"))?;
    measure_against("3.3.7", 3, 0, Kumihan::new(policy))
}

#[test]
fn section_3_3_7_is_also_measured_under_katatsuki() -> Outcome {
    // The identical second-run shape `section_3_3_5_is_also_measured_under_katatsuki` already
    // gives §3.3.5 directly, applied here to §3.3.7¶1's own delegation to it and to the
    // alignment-discouraged fact §3.3.7¶1 carries along with that delegation
    // (`crates/jlreq-inline/src/lower.rs`'s own `Contribution::alignment_discouraged` doc): a
    // `Kumihan` declaring `ruby.alignment: katatsuki` selects `3.3.7/jukugo-ruby-placement/
    // paragraph-one-per-base-mono-delegation`'s own second entry and `3.3.7/jukugo-ruby-
    // alignment/katatsuki-discouraged-carries-through-the-delegation`'s own second entry
    // alike, over the identical case file, so both are attempted and agreed a second time
    // under the other reading. The paragraph-2 case is unaffected — `place_jukugo_compound`
    // never reads `Question::RUBY_ALIGNMENT` at all — so the census below is identical to
    // `section_3_3_7`'s own, not merely close to it.
    let katatsuki = choice(Question::RUBY_ALIGNMENT, "katatsuki")?;
    let policy = Policy::JLREQ
        .with(katatsuki)
        .map_err(|conflict| format!("{conflict:?}"))?;
    measure_against("3.3.7", 3, 0, Kumihan::new(policy))
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
/// Stage 2 of the policy-space derivation has now run: `spec/derived/questions.tsv`
/// inventories twenty-two places JLReq permits more than one answer, and every one of them
/// is a `Question` constant `Policy::JLREQ` answers. A permitted entry naming any of the
/// twenty-two is therefore selectable, and the figure that used to count every entry naming
/// a question the policy space did not have yet (170, before stage 2 emitted
/// `crates/jlreq-spec/src/generated/policy.rs`) fell to zero the moment it did.
///
/// The number is committed rather than reported, because a run in which it changes is a run
/// in which the suite proves something different: it falls whenever a case gains a reading
/// this workspace could not previously be measured against, and it rises whenever a case
/// publishes a reading naming a question the twenty-two still do not cover. Either is a
/// reviewable act.
const UNSELECTABLE: usize = 0;

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
