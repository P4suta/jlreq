#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The tab-separated files under `spec/`.
;;
;; Both families this engine reads -- the derived tables under `spec/derived/` and
;; the transcribed matrices under `spec/captured/` -- use one shape:
;;
;; - a preamble of `#` comment lines, then a blank line;
;; - one header line naming the columns;
;; - one data line per row, fields separated by U+0009.
;;
;; A field never holds a raw tab or a raw newline. Prose fields (the Remarks
;; column of Appendix A is the only one in this engine's inputs) escape a line
;; break as `\n` and a backslash as `\\`, and no other escape exists: an unknown
;; escape is refused rather than passed through, because a field nothing writes is
;; a field nobody can read.
;;
;; Blank lines and `#` lines are dropped wherever they appear, not only in the
;; preamble, so the reader does not depend on a fixed number of header comments --
;; the derived files carry a source digest in their preamble and gain a line
;; whenever the tooling that writes them changes.

(require racket/string racket/vector)

(provide (struct-out tsv)
         tsv-parse
         tsv-column
         tsv-row-count
         tsv-field
         tsv-field-of
         tsv-unescape)

;; `header` is a vector of column names; `rows` is a list of equal-length vectors,
;; in file order.
(struct tsv (header rows) #:transparent)

(define (fail template . arguments)
  (raise (exn:fail (apply format template arguments) (current-continuation-marks))))

;; One field with its escapes resolved.
(define (tsv-unescape field)
  (if (not (regexp-match? #rx"\\\\" field))
      field
      (let ([out (open-output-string)] [size (string-length field)])
        (let step ([index 0])
          (cond
            [(>= index size) (void)]
            [(not (char=? (string-ref field index) #\\))
             (write-char (string-ref field index) out)
             (step (add1 index))]
            [(>= (add1 index) size) (fail "`~a` ends in a backslash, which is not an escape" field)]
            [else
             (case (string-ref field (add1 index))
               [(#\n)
                (write-char #\newline out)
                (step (+ index 2))]
               [(#\\)
                (write-char #\\ out)
                (step (+ index 2))]
               [else
                (fail "`~a` holds the escape `\\~a`, which nothing writes"
                      field
                      (string (string-ref field (add1 index))))])]))
        (get-output-string out))))

;; `line` split at every U+0009, keeping empty fields at both ends.
(define (split-tabs line)
  (list->vector (string-split line "\t" #:trim? #f)))

;; `text` split at every U+000A, with a trailing U+000D removed from each line so a
;; file checked out with CRLF endings still reads.
(define (text-lines text)
  (for/list ([line (in-list (string-split text "\n" #:trim? #f))])
    (if (and (> (string-length line) 0) (char=? (string-ref line (sub1 (string-length line))) #\return))
        (substring line 0 (sub1 (string-length line)))
        line)))

;; Whether a line carries data: not blank, not a comment.
(define (data-line? line)
  (and (> (string-length line) 0)
       (not (char=? (string-ref line 0) #\#))
       (not (string=? (string-trim line) ""))))

;; Parse a whole file. Raises when the file has no header line, or when a row's
;; field count differs from the header's.
(define (tsv-parse text)
  (define significant (filter data-line? (text-lines text)))
  (when (null? significant)
    (fail "the file holds no header line"))
  (define header (vector-map tsv-unescape (split-tabs (car significant))))
  (define width (vector-length header))
  (define rows
    (for/list ([line (in-list (cdr significant))] [index (in-naturals 1)])
      (define fields (split-tabs line))
      (unless (= (vector-length fields) width)
        (fail "data row ~a has ~a field(s) where the header names ~a"
              index
              (vector-length fields)
              width))
      (vector-map tsv-unescape fields)))
  (tsv header rows))

;; The index of the column named `name`. Raises when the file has no such column.
(define (tsv-column file name)
  (define header (tsv-header file))
  (let search ([index 0])
    (cond
      [(>= index (vector-length header)) (fail "the file has no column named `~a`" name)]
      [(string=? (string-trim (vector-ref header index)) name) index]
      [else (search (add1 index))])))

;; How many data rows the file holds.
(define (tsv-row-count file)
  (length (tsv-rows file)))

;; One field of one row. Rows are already known to be the header's width, so this
;; only fires on a programming error.
(define (tsv-field row index)
  (vector-ref row index))

;; The field of `row` in the column named `name`. Convenient where a file is read
;; once and the column indices are not worth naming.
(define (tsv-field-of file row name)
  (tsv-field row (tsv-column file name)))
