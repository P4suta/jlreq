# ADR-0009: generated where the specification is machine-readable, attested where it is not

- Status: accepted
- Date: 2026-08-05

## Context

[CONTRIBUTING.md](../../CONTRIBUTING.md) states that specification data is generated, not
transcribed, and that a hand-edited table entry is a bug even when it is correct. The rule
is right and it collides with how W3C publishes JLReq.

Appendix A is machine-readable: 25 HTML tables, 1133 keys, a Remarks column, and section
anchors. Generating it is straightforward.

Appendices B through E are not. Measured in the published document, the range containing
those four appendices holds prose, a legend, a notes list, and an anchor of the literal
form `See "…" (PDF)` — and exactly one `<table>` element, which renders the two
substitution examples inside §B.2 note 14. Six matrices totaling roughly 5400 cells exist
only as PDF. Worse, the reduction and expansion priority ordinals are encoded as cell
background color, and the color-to-ordinal key is a raster image, so the ordering exists
nowhere as text.

Claiming to generate those matrices by scraping a PDF would produce a script nobody can
validate, whose most important derived fact is still read by eye, and whose failure mode is
a plausible wrong number presented as machine-derived. That is worse than an honest
transcription, because it suppresses the scrutiny a transcription invites.

The published document also carries defects a generator must surface rather than absorb. A
code point is listed twice in cl-19, §D.2 note 5 contradicts notes 1 through 3 on a
priority ordinal, §3.1.3's closing note reads "vertical" in English against 横組 in
Japanese, and the legend anchors are off by one from the table numbers they render. There
are nine such, each recorded with its evidence.

## Decision

The rule is split in two, and the second half is given a control strong enough to earn its
keep.

Derived data — Appendix A, the legends, every appendix note, the strictness levels, the
adjustment ladders, the rule inventory, the ideograph predicate, the compatibility folding
table — is generated from the HTML snapshot and vendored Unicode Character Database
extracts. A hand edit to a generated file is a bug, detected by regenerating and comparing
bytes.

Captured data — the cells of Tables 1 through 6 — is transcribed, and the transcription is
the primary source in this repository. It is confined to one directory, and it is entered
twice, independently, from the English and the Japanese rendering of the same matrix, which
W3C publishes as separate documents. Every cell records the source file, table, row label,
and column label; a cell without provenance fails the build. The two entries must agree
cell for cell.

Double entry catches a slip and a systematic error survives it, so it is named for what it
is — a procedural control — and the mechanical one is a set of cross-table invariants
derived from prose that *is* machine-readable: that §E.1's blank means unexpandable because
Table 2 forbids a break, that §D.1's unadjusted amounts are Table 1's, that §3.1.7's ten
line-start-prohibited classes are exactly the columns Table 2 forbids at the strictest
level, and thirteen more. Each cites the sentence that justifies it and each is also a
conformance case, so the redundancy is published rather than private.

Every recorded defect is data. If a defect is fixed upstream the gate fails and forces a
review, rather than the behavior changing quietly.

## Consequences

CONTRIBUTING.md must be amended, and the amendment narrows the rule honestly instead of
weakening it: generated where the specification is machine-readable, attested where it is
not, and `crates/**/src/**` never hand-edited either way.

Roughly 5400 cells must be keyed twice by a human. That is the largest single labor item in
the project and no design removes it. What the design buys is that the work is checkable.

If W3C ever publishes machine-readable appendices, the captured directory becomes a
regression fixture and the split disappears.
