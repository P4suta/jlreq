# Roadmap

Each milestone is independently useful. Nothing here needs hardware, a license, a font
file, or a network: every milestone is verifiable by `cargo test` alone.

## M0 — Character classes

`jlreq-class`: determine the JLReq class (cl-01 … cl-30) of an occurrence — a cluster of
the caller's text, together with the character frame (字幅) its supplied advance covers
and the role the document gives it — generated from the published tables rather than
transcribed, plus the conformance cases for it.

Of an occurrence and not of a code point: 473 of Appendix A's 1133 keys are named by more
than one class, and five classes enumerate no character at all, so the total function from
a code point to a class does not exist to be written
([ADR 0008](docs/adr/0008-classification-is-a-function-of-an-occurrence.md)). Where the
supplied facts do not separate the surviving candidates, the answer names the candidates
and the axis that would separate them, rather than guessing.

Under it sit the two vocabulary crates every later milestone speaks through: `jlreq-unit`
for quantities, axes, and items, and `jlreq-spec` for the specification addresses every
answer cites.

No other implementation exposes this as a callable library, so M0 stands on its own.

## M1 — Kinsoku and line adjustment

`jlreq-line`: line-start and line-end prohibition, non-separation rules, oikomi (追い込み)
and oidashi (追い出し). At this point Japanese text wraps correctly — `、` and `。` stop
appearing at the start of a line.

## M2 — Mojikumi

`jlreq-spacing`: spacing between punctuation, brackets, and ideographs. Equivalent to the
CSS `text-spacing-trim` property, which JLReq specifies. This is the first milestone whose
result is visible in a screenshot without explanation.

## M3 — Paragraph optimization

Whole-paragraph line breaking rather than greedy, in the Knuth–Plass sense, with hanging
punctuation (ぶら下げ) as an adjustment option: a stage of the adjustment ladder, between
the reduction stages and the expansion stages, which is where JLReq puts it. A line that
fits without hanging does not hang (§2.5.1), and a line that would otherwise be expanded
hangs first (§3.8.2) — so hanging is not a repair applied after a break has been chosen,
and the greedy search and the optimal one cannot disagree about *whether a fixed range
hangs* (both drain the identical ladder, in the identical order, once a line's own start and
end are the same for each). They can and do disagree about *which ranges exist to ask the
question of*: `FirstFit` picks a line's own end from unadjusted geometry alone, blind to
what the choice costs the next line, so it can choose a different sequence of breaks than
`Optimal`'s own paragraph-wide minimization does — and a character that hangs under one
search's own chosen line need not even be that line's own last character under the other's.
`jlreq_line::compose`'s own test suite carries a constructed pair of paragraphs that
disagree exactly this way, one on which the two searches produce different line counts and
different total demerits, and a second where a trailing full stop hangs under `Optimal` but
sits comfortably mid-measure — never overfull, never offered to `ladder::hang` at all —
under `FirstFit`.

## M4 — Inline constructs

`jlreq-inline`: ruby (mono, group, and jukugo), tate-chu-yoko (縦中横), emphasis dots
(圏点), warichu (割注), furiwake (振分け, §3.7.2), jidori (字取り, §3.7.3), reference
marks (合印, §4.2.3), the ornamented character complex (cl-21, §3.7.1), and math and
chemical formulae (§3.7.4), including their effect on line spacing.

Nine constructs rather than four, because
[ADR 0013](docs/adr/0013-rules-are-addressed-by-specification-address.md)'s coverage gate
is set subtraction over the rule inventory in both directions, and it cannot close on a
milestone that leaves five normative processes unimplemented. The growth is cheap
precisely because the nine share one mechanism: each is declared by the caller, lowered
across the single seam between `jlreq-inline` and `jlreq-line`, and placed against the
space that survived line adjustment. The ninth construct costs a declaration and its
conformance cases rather than a code path.

## M5 — Vertical writing

Vertical composition through the writing-direction abstraction established in
[ADR 0004](docs/adr/0004-writing-mode-abstraction.md) — the same code path, not a parallel
implementation.

## M6 — Adoption

An adapter for one downstream consumer, to prove the boundary holds in practice.
Candidates: Typst (no vertical writing or ruby as of 0.14), Parley, cosmic-text.

## Non-goals

Font loading, shaping, rasterization, and file I/O are permanently out of scope
([ADR 0001](docs/adr/0001-no-std-no-io-no-font-in-core.md)). Chinese (CLReq) and Korean
support are plausible later extensions because the rule structure is shared, but they are
not on this roadmap and will not be allowed to distort the Japanese model.
