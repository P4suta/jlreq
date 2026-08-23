#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The specification tables: the startup census, the legend grammar, and the
;; comparison of the two transcriptions of Appendices B through E.
;;
;; The last of those is the reason this file is longer than the others. The engine
;; runs on `spec/captured/table*.en.tsv`; this compares all 4,932 cells and their
;; note citations against `table*.ja.tsv`, which is a third independent reading of
;; the same six PDF pages and turns a transcription divergence into a named
;; coordinate in a test failure.

(require rackunit
         racket/list
         "../tables.rkt"
         (prefix-in ja: "ja-data.rkt"))

(module+ test
  ;; ------------------------------------------------------------------
  ;; The startup census
  ;; ------------------------------------------------------------------

  (check-not-exn self-check "the tables are the size and shape spec/ states")

  (check-equal? (length appendix-a) 1687)
  (check-equal? (hash-count appendix-a-listings) 1686)
  (check-equal? (hash-count appendix-a-keys) 1133)
  (check-equal? (length appendix-a-remarks) 14)
  (check-equal? (length folding) 226)
  (check-equal? (length ideographs) 16)
  (check-equal? (length scripts) 22)
  (check-equal? (length classes) 30)

  ;; Exactly one (class, key) listing is stated twice. `cl-19 216B` is a defect of
  ;; the published document that spec/derived/defects.tsv records rather than
  ;; resolves, and the gap between 1,687 rows and 1,686 listings is where it shows.
  (check-equal? (- (length appendix-a) (hash-count appendix-a-listings)) 1)
  (check-equal? (length (hash-ref appendix-a-keys (list #x216B))) 2)

  ;; ------------------------------------------------------------------
  ;; Class labels and amounts
  ;; ------------------------------------------------------------------

  (check-equal? (class-of-label "cl-01") 1)
  (check-equal? (class-of-label "cl-30") 30)
  (check-equal? (class-of-label "line-head") 0)
  (check-equal? (class-of-label "line-end") 0)
  (check-exn exn:fail? (lambda () (class-of-label "cl-31")))
  (check-exn exn:fail? (lambda () (class-of-label "cl-1")))
  (check-exn exn:fail? (lambda () (class-of-label "")))
  (check-equal? (row-label 0) "line-head")
  (check-equal? (column-label 0) "line-end")
  (check-equal? (row-label 7) "cl-07")
  (check-equal? (column-label 30) "cl-30")

  ;; cl-17 and cl-18 are classes Appendix A enumerates and no matrix axis carries.
  (check-true (class? 17))
  (check-false (has-adjacency? 17))
  (check-false (has-adjacency? 18))
  (check-false (axis-class? 17))
  (check-true (axis-class? 0))
  (check-true (axis-class? 30))

  (check-equal? unit-per-em 720)
  (check-equal? (amount-of-token "0") 0)
  (check-equal? (amount-of-token "1/8") 90)
  (check-equal? (amount-of-token "1/4") 180)
  (check-equal? (amount-of-token "1/2") 360)
  (check-equal? (amount-of-token "3/4") 540)
  (check-equal? (amount-of-token "1") 720)
  (check-exn exn:fail? (lambda () (amount-of-token "2")) "an amount above one em is refused")
  (check-exn exn:fail? (lambda () (amount-of-token "1/7")) "1/7 em is not exact in 1/720 em")
  (check-exn exn:fail? (lambda () (amount-of-token "1/0")))

  ;; ------------------------------------------------------------------
  ;; The legend-token grammar
  ;; ------------------------------------------------------------------

  (check-equal? (parse-cell "blank") 'blank)
  (check-equal? (parse-cell "") 'blank)
  (check-equal? (parse-cell "×") 'prohibited)
  (check-equal? (parse-cell "ruby hang") 'ruby-hang)
  (check-equal? (parse-cell "residual") 'residual)
  (check-equal? (parse-cell "not") (no-break '()) "a bare `not` is every C.3 level")
  (check-equal? (parse-cell "not 3,4") (no-break '(3 4)))
  (check-equal? (parse-cell "1/4 be") (spacing (list (term 180 'before #f))))
  (check-equal? (parse-cell "1/2 af") (spacing (list (term 360 'after #f))))
  (check-equal? (parse-cell "1/4 be hang") (spacing (list (term 180 'before #t))))
  (check-equal? (parse-cell "1/2 be + 1/4 af")
                (spacing (list (term 360 'before #f) (term 180 'after #f))))
  (check-equal? (parse-cell "1/2") (rigid 360 #f))
  (check-equal? (parse-cell "1/4 stage 3") (rigid 180 3))
  (check-equal? (parse-cell "1/4-1/8 stage 6") (movable 180 90 #f 6))
  (check-equal? (parse-cell "1/2=0 stage 2") (movable 360 0 #t 2) "`=` is 3.1.9's two-valued form")
  (check-equal? (parse-cell "0-1/4 stage 3") (movable 0 180 #f 3))
  ;; A hyphen, an en dash and an em dash are the same separator.
  (check-equal? (parse-cell "1/4–1/8 stage 6") (movable 180 90 #f 6))
  (check-equal? (parse-cell "1/4—1/8 stage 6") (movable 180 90 #f 6))
  (check-exn exn:fail? (lambda () (parse-cell "1/4 up")) "a side outside the legend is refused")
  (check-exn exn:fail? (lambda () (parse-cell "1/4-1/8")) "a limit with no stage is refused")
  (check-exn exn:fail? (lambda () (parse-cell "1/4 stage 12")))

  ;; ------------------------------------------------------------------
  ;; The six matrices
  ;; ------------------------------------------------------------------

  (check-equal? (vector-length matrices) 6)
  (for ([number (in-range 1 7)])
    (define table (matrix-of number))
    (define axis (if (or (= number 2) (= number 6)) 28 29))
    (check-equal? (hash-count (matrix-cells table)) (* axis axis) (format "Table ~a cells" number))
    (check-equal? (vector-length (matrix-row-axis table)) axis)
    (check-equal? (vector-length (matrix-column-axis table)) axis))

  ;; Four coordinates read straight back out of the published tables.
  (check-equal? (cell-at (matrix-of 1) 1 5) (spacing (list (term 180 'after #f)))
                "Table 1 (cl-01, cl-05): a quarter em from the middle dot's own em")
  (check-equal? (cell-at (matrix-of 1) 1 0) 'prohibited
                "Table 1 (cl-01, line-end): an opening bracket may not end a line")
  (check-equal? (cell-at (matrix-of 2) 1 2) (no-break '())
                "Table 2 (cl-01, cl-02): no break opportunity at any C.3 level")
  (check-equal? (cell-at (matrix-of 6) 22 27) (movable 180 360 #f 2)
                "Table 6 (cl-22, cl-27): a quarter em expandable to a half, stage 2")

  ;; ------------------------------------------------------------------
  ;; The two transcriptions of Appendices B through E
  ;; ------------------------------------------------------------------

  ;; The engine runs on the English side. This builds the Japanese one and compares
  ;; every cell and every note citation. The two files are keyed in different row
  ;; orders, so the comparison is by coordinate.
  (define japanese
    (vector (parse-matrix 1 ja:table1)
            (parse-matrix 2 ja:table2)
            (parse-matrix 3 ja:table3)
            (parse-matrix 4 ja:table4)
            (parse-matrix 5 ja:table5)
            (parse-matrix 6 ja:table6)))

  (define compared
    (for/sum ([number (in-range 1 7)])
      (define english (matrix-of number))
      (define other (vector-ref japanese (sub1 number)))
      (check-equal? (matrix-row-axis other) (matrix-row-axis english)
                    (format "Table ~a row axis, English against Japanese" number))
      (check-equal? (matrix-column-axis other) (matrix-column-axis english)
                    (format "Table ~a column axis, English against Japanese" number))
      (for*/sum ([before (in-vector (matrix-row-axis english))]
                 [after (in-vector (matrix-column-axis english))])
        (check-equal? (cell-at other before after)
                      (cell-at english before after)
                      (format "Table ~a (~a, ~a)" number (row-label before) (column-label after)))
        (check-equal? (hash-ref (matrix-notes other) (coordinate before after) "")
                      (hash-ref (matrix-notes english) (coordinate before after) "")
                      (format "Table ~a (~a, ~a) note" number (row-label before) (column-label after)))
        1)))

  (check-equal? compared 4932 "every cell of all six matrices was compared across the two locales"))
