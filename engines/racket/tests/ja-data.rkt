#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The Japanese transcriptions of Appendices B through E, embedded here and
;; nowhere else.
;;
;; The engine reads the English ones (see ../specdata.rkt); this module gives the
;; tests the other transcription so that test-tables.rkt can compare them cell for
;; cell. The OCaml engine reads the Japanese side and its own tests compare the
;; English one, so this is the mirror image of `engines/ocaml/test/test_tables.ml`
;; -- and either way a disagreement is a transcription defect that would otherwise
;; show up as an unexplained DIFF nine milestones from now.
;;
;; The two locales' files are keyed in different row orders, so the comparison is
;; by coordinate and never by row.

(require "../embed.rkt")

(provide table1 table2 table3 table4 table5 table6)

(define table1 (embed-file "../../../spec/captured/table1.ja.tsv"))
(define table2 (embed-file "../../../spec/captured/table2.ja.tsv"))
(define table3 (embed-file "../../../spec/captured/table3.ja.tsv"))
(define table4 (embed-file "../../../spec/captured/table4.ja.tsv"))
(define table5 (embed-file "../../../spec/captured/table5.ja.tsv"))
(define table6 (embed-file "../../../spec/captured/table6.ja.tsv"))
