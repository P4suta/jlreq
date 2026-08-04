# Roadmap

Each milestone is independently useful. Nothing here needs hardware, a license, a font
file, or a network: every milestone is verifiable by `cargo test` alone.

## M0 — Character classes

`jlreq-class`: determine the JLReq class (cl-1 … cl-30) for a code point, generated from
the published tables rather than transcribed, plus the conformance cases for it.

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
punctuation (ぶら下げ) as an adjustment option.

## M4 — Inline constructs

`jlreq-inline`: ruby (mono, group, and jukugo), tate-chu-yoko (縦中横), emphasis dots, and
warichu, including their effect on line spacing.

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
