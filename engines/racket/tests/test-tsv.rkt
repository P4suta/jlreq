#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The TSV reader, against the shape spec/ actually writes.

(require rackunit "../tsv.rkt")

(module+ test
  (define simple "# a preamble line\n#\n\nname\tvalue\none\t1\ntwo\t2\n")
  (define file (tsv-parse simple))

  (check-equal? (vector->list (tsv-header file)) '("name" "value"))
  (check-equal? (tsv-row-count file) 2)
  (check-equal? (tsv-column file "value") 1)
  (check-equal? (tsv-field (car (tsv-rows file)) 0) "one")
  (check-equal? (tsv-field-of file (cadr (tsv-rows file)) "value") "2")
  (check-exn exn:fail? (lambda () (tsv-column file "missing")))

  ;; Comment and blank lines are dropped wherever they appear, not only in the
  ;; preamble: the derived files gain a header line whenever the tooling that
  ;; writes them changes.
  (check-equal? (tsv-row-count (tsv-parse "a\tb\n\n# in the middle\nx\ty\n")) 1)

  ;; An empty field at either end survives the split.
  (define edges (tsv-parse "a\tb\tc\n\t1\t\n"))
  (check-equal? (vector->list (car (tsv-rows edges))) '("" "1" ""))

  ;; A file checked out with CRLF endings still reads.
  (check-equal? (vector->list (tsv-header (tsv-parse "a\tb\r\n1\t2\r\n"))) '("a" "b"))

  ;; A short row is a corrupt file, not a row with a missing column.
  (check-exn exn:fail? (lambda () (tsv-parse "a\tb\tc\n1\t2\n")))
  (check-exn exn:fail? (lambda () (tsv-parse "# nothing but a preamble\n")))

  ;; The two escapes that exist, and the refusal of everything else.
  (check-equal? (tsv-unescape "plain") "plain")
  (check-equal? (tsv-unescape "one\\ntwo") "one\ntwo")
  (check-equal? (tsv-unescape "a\\\\b") "a\\b")
  (check-exn exn:fail? (lambda () (tsv-unescape "a\\tb")) "an escape nothing writes is refused")
  (check-exn exn:fail? (lambda () (tsv-unescape "trailing\\")))

  ;; The escapes are resolved by the parser and not left to the caller.
  (define escaped (tsv-parse "a\tb\nx\\ny\tz\n"))
  (check-equal? (tsv-field-of escaped (car (tsv-rows escaped)) "a") "x\ny"))
