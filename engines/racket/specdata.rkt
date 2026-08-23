#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; Layer 0: the specification tables, as strings.
;;
;; Each name below is one file of `spec/`, pasted in at compile time by
;; `embed-file` (see embed.rkt for why it is a paste and not a path). Nothing here
;; parses anything; tables.rkt does that, and does it at startup from these
;; strings.
;;
;; The captured Appendix B-E matrices are read in ENGLISH. The Rust engine reads
;; the English ones too; the OCaml engine reads the Japanese ones. The pairing is
;; what makes the transcription cross-check hold across three engines rather than
;; two: `engines/ocaml` answering the same eighty-nine cases from `table*.ja.tsv`
;; is the evidence that the two hand transcriptions of those six PDF pages agree,
;; and this engine additionally compares the Japanese side against the English one
;; cell for cell in `tests/test-tables.rkt` -- the mirror image of the comparison
;; `engines/ocaml/test/test_tables.ml` makes -- so a divergence surfaces as a named
;; coordinate in a test failure rather than as an unexplained DIFF nine milestones
;; from now. "The independence rule" of docs/design/conformance.md describes the
;; two-engine case, where the second engine is the one that reads the other locale;
;; with a third engine the locales cannot all differ, and what has to stay true is
;; that both transcriptions are read by something.

(require "embed.rkt")

(provide appendix-a
         folding
         ideographs
         scripts
         classes
         questions
         table1
         table2
         table3
         table4
         table5
         table6)

;; spec/derived/ -- generated from the W3C snapshot and the character database by
;; `cargo run -p xtask -- derive`, and attested by `just attest`.
(define appendix-a (embed-file "../../spec/derived/appendix-a.tsv"))
(define folding (embed-file "../../spec/derived/folding.tsv"))
(define ideographs (embed-file "../../spec/derived/ideographs.tsv"))
(define scripts (embed-file "../../spec/derived/scripts.tsv"))
(define classes (embed-file "../../spec/derived/classes.tsv"))
(define questions (embed-file "../../spec/derived/questions.tsv"))

;; spec/captured/ -- the six matrices of Appendices B through E, which W3C
;; publishes only as PDF, transcribed by hand (ADR 0009).
(define table1 (embed-file "../../spec/captured/table1.en.tsv"))
(define table2 (embed-file "../../spec/captured/table2.en.tsv"))
(define table3 (embed-file "../../spec/captured/table3.en.tsv"))
(define table4 (embed-file "../../spec/captured/table4.en.tsv"))
(define table5 (embed-file "../../spec/captured/table5.en.tsv"))
(define table6 (embed-file "../../spec/captured/table6.en.tsv"))
