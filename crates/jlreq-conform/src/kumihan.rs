// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! This workspace, as one implementation of [`Compose`].
//!
//! The suite is written to be run against anyone's implementation, so the workspace's own
//! is an adapter like every other: it builds the `Text` a case's input describes, asks
//! `jlreq` the question the case asks, and reports `None` for a question this workspace
//! does not answer yet. Nothing here knows what a case expects.
//!
//! # What is answered today, and what is not
//!
//! Classification is answered. Boundary and composition are not: `jlreq-spacing` and
//! `jlreq-line` have no evaluator at M0, so both methods report `None` — *not attempted* —
//! which is exactly what the trait's `Option` is for. A count of skipped cases is an honest
//! statement about a milestone; a fabricated answer would not be.
//!
//! One further `None` is a statement about the layer rather than about the schedule.
//! `classify` takes a text, an ordinal and a policy, and takes no construct: a construct is
//! a run over a stream rather than a property of one item, so it belongs to `jlreq-inline`
//! (ADR 0015). Where a case declares a construct that runs over the very occurrence it asks
//! about, the answer this crate could give would be an answer to a different question — it
//! would be the class the occurrence has read as bare running text, which is not what the
//! case asked. Nine of the thirty classes are membership *in* a construct, five of them
//! enumerate no keys at all, and neither fact is reachable from an item. So the occurrence
//! is reported as not attempted, and the rule is stated once, here, rather than as a list of
//! case ids.

use jlreq::{
    Advance, ByteOffset, Frame, InlineExtent, Item, ItemIndex, Role, Scale, ScaleId, Text,
    TextError,
};
// The specification's own vocabulary comes from `jlreq-spec` rather than through the
// facade, which is the reason the crate graph gives this crate both edges: the suite has to
// reach the rule inventory and the policy space to report coverage without depending on the
// whole of the layout stack for it (`docs/design/api-spine.md`).
use jlreq_spec::{Choice, Policy, Question};

use crate::case::{CaseInput, CaseItem, CasePolicy, Suite};
use crate::run::{CaseBoundary, CaseClass, CaseOutput, Compose};

/// This workspace's implementation, as the suite measures it.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Kumihan {
    /// The policy this run declares.
    policy: Policy,
}

impl Kumihan {
    /// The workspace under one policy.
    #[must_use]
    pub const fn new(policy: Policy) -> Self {
        Self { policy }
    }
}

impl Default for Kumihan {
    /// JLReq's own preference wherever it states one, which is the preset the library's own
    /// documentation is written against.
    fn default() -> Self {
        Self::new(Policy::JLREQ)
    }
}

impl Compose for Kumihan {
    fn name(&self) -> &'static str {
        "kumihan"
    }

    /// Every question of the generated policy space, answered as this run's policy answers
    /// it.
    ///
    /// The map is total over the questions that exist, which is what the selection rule
    /// reads. Where the policy space is smaller than `spec/derived/questions.tsv` — stage 2
    /// of the derivation emits the `Question` constants and has not run — the map is
    /// correspondingly smaller, and a case entry keyed on a question this workspace cannot
    /// yet answer applies to nothing. That is the honest reading: the entry names a knob
    /// this implementation does not have, so it is not the entry this implementation is
    /// measured against.
    fn declared_policy(&self) -> Option<CasePolicy> {
        Some(
            Question::ALL
                .iter()
                .map(|question| {
                    (
                        question.path().to_owned(),
                        Choice::name(self.policy.get(*question)).to_owned(),
                    )
                })
                .collect(),
        )
    }

    fn classify(&self, input: &CaseInput, item: usize) -> Option<CaseClass> {
        if input.construct_covers(item) {
            return None;
        }
        let stream = Stream::of(input).ok()?;
        let answer = jlreq::resolve(stream.text().ok()?, ordinal(item)?, self.policy)?;
        Some(CaseClass {
            class: answer.value().number(),
            rules: answer
                .why()
                .rules()
                .map(|rule| rule.address().to_string())
                .collect(),
        })
    }

    /// Not attempted: `jlreq-spacing` has no evaluator at M0, so there is no boundary answer
    /// to report and inventing one would publish a reading nothing produced.
    fn boundary(&self, _input: &CaseInput, _before: usize) -> Option<CaseBoundary> {
        None
    }

    /// Not attempted, for the reason `boundary` is not: `jlreq-line` composes nothing yet.
    fn compose(&self, _input: &CaseInput) -> Option<CaseOutput> {
        None
    }
}

/// One case's base stream, in the library's own vocabulary.
///
/// Held as a value rather than built inline because `Text` borrows the items and the scales,
/// so both have to outlive it.
#[derive(Debug)]
#[non_exhaustive]
pub struct Stream<'a> {
    /// The stream's own text.
    text: &'a str,
    /// One item per occurrence.
    items: Vec<Item>,
    /// The character sizes the stream declares.
    scales: Vec<Scale>,
}

impl<'a> Stream<'a> {
    /// Read one case's base stream, or say which part of it the vocabulary cannot hold.
    ///
    /// A case that reaches here has already passed `conform --check`, which holds the same
    /// input to the same schema, so a failure here is a disagreement between two readings of
    /// one format rather than a malformed case.
    pub fn of(input: &'a CaseInput) -> Result<Self, String> {
        let scales = input
            .scales
            .iter()
            .map(|scale| {
                let inline = advance(scale.inline_em)?;
                let block = advance(scale.block_em)?;
                Scale::new(inline, block).ok_or_else(|| "a character size is positive".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let items = input
            .items
            .iter()
            .map(item_of)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            text: &input.text,
            items,
            scales,
        })
    }

    /// The stream this crate's own constructor accepts, or the refusal it answered with.
    pub fn text(&self) -> Result<Text<'_>, TextError> {
        Text::new(self.text, &self.items, &self.scales)
    }
}

/// One item in the library's own vocabulary.
fn item_of(item: &CaseItem) -> Result<Item, String> {
    let start = u32::try_from(item.start).map_err(|_| "an offset is a byte of the stream")?;
    let width = i32::try_from(item.advance).map_err(|_| "an advance is a length".to_owned())?;
    let advance =
        InlineExtent::new(width).ok_or_else(|| "an advance is not negative".to_owned())?;
    let scale = u8::try_from(item.scale).map_err(|_| "a stream declares at most 32 sizes")?;
    Ok(
        Item::new(ByteOffset::new(start), advance, ScaleId::new(scale))
            .with_frame(frame_of(item.frame.as_deref()))
            .with_role(role_of(item.role.as_deref())),
    )
}

/// One length in the library's own vocabulary.
fn advance(value: i64) -> Result<Advance, String> {
    let units = i32::try_from(value).map_err(|_| "a length is an i32 of caller units")?;
    Advance::new(units).ok_or_else(|| "a length is not negative".to_owned())
}

/// The frame a case names, which is the schema's own vocabulary.
fn frame_of(frame: Option<&str>) -> Frame {
    match frame {
        Some("full-em") => Frame::FullEm,
        Some("half-em") => Frame::HalfEm,
        Some("third-em") => Frame::ThirdEm,
        Some("quarter-em") => Frame::QuarterEm,
        Some("proportional") => Frame::Proportional,
        _ => Frame::Unstated,
    }
}

/// The role a case names, which is the schema's own vocabulary.
fn role_of(role: Option<&str>) -> Role {
    match role {
        Some("decimal-point") => Role::DecimalPoint,
        Some("digit-group-separator") => Role::DigitGroupSeparator,
        Some("unit-symbol") => Role::UnitSymbol,
        Some("quantity-symbol") => Role::QuantitySymbol,
        Some("sentence-terminator") => Role::SentenceTerminator,
        Some("sentence-medial") => Role::SentenceMedial,
        _ => Role::Unstated,
    }
}

/// One ordinal in the library's own vocabulary.
fn ordinal(item: usize) -> Option<ItemIndex> {
    u32::try_from(item).ok().map(ItemIndex::new)
}

/// Every case input this workspace's own `Text::new` refuses.
///
/// `conform --check` and `Text::new` are two implementations of ADR 0018's invariants — one
/// over the published format, one over the library's constructor — so a case the gate
/// accepts and the constructor refuses is a divergence between them and a finding in its own
/// right, never a case to skip. It is reported here rather than folded into the run, because
/// a refusal is not an answer.
#[must_use]
pub fn refusals(suite: &Suite) -> Vec<String> {
    let mut found = Vec::new();
    for case in suite.cases() {
        match Stream::of(case.input()).and_then(|stream| {
            stream
                .text()
                .map(|_| ())
                .map_err(|error| format!("Text::new refused it: {error:?}"))
        }) {
            Ok(()) => {},
            Err(reason) => found.push(format!("{id}: {reason}", id = case.id())),
        }
    }
    found
}
