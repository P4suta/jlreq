#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The specification tables, built from `spec/` at startup.
;;
;; `self-check` is called from main.rkt before the first request is read, and a
;; failure exits 2.
;;
;; The census is not decoration. Every number in `self-check` was measured against
;; the files in `spec/` and is a claim about the specification, so a change in
;; `spec/` that silently drops rows stops the engine at startup instead of
;; producing a subtly wrong layout eighty-nine times. It is also the only defense
;; this engine has against `embed-file` pasting a truncated file: a short paste
;; would still compile.
;;
;; Amounts
;;
;; Every spacing amount is an exact multiple of 1/720 em, which is the unit the six
;; matrices are transcribed in (`amounts-are-multiples-of-the-unit`, ADR 0007). A
;; quarter em is 180, a half em is 360, a full em is 720. Nothing here is a
;; fraction at run time.
;;
;; Locale
;;
;; The captured matrices are read from the ENGLISH transcriptions; see specdata.rkt
;; for why, and `tests/test-tables.rkt` for the comparison against the Japanese
;; ones that keeps the divergences the capture preambles record visible.

(require racket/list
         racket/string
         "tsv.rkt"
         (prefix-in data: "specdata.rkt"))

(provide unit-per-em
         max-amount
         max-stage
         class?
         has-adjacency?
         axis-class?
         class-of-label
         row-label
         column-label
         amount-of-token
         (struct-out listing)
         (struct-out range-entry)
         (struct-out fold)
         (struct-out script-range)
         (struct-out class-entry)
         (struct-out term)
         (struct-out no-break)
         (struct-out spacing)
         (struct-out rigid)
         (struct-out movable)
         (struct-out matrix)
         appendix-a
         appendix-a-listings
         appendix-a-keys
         appendix-a-remarks
         folding
         ideographs
         scripts
         classes
         questions
         matrices
         matrix-of
         coordinate
         cell-at
         states?
         parse-cell
         parse-matrix
         self-check)

(define (fail template . arguments)
  (raise (exn:fail (apply format template arguments) (current-continuation-marks))))

;; ----------------------------------------------------------------------------
;; Amounts and class identifiers
;; ----------------------------------------------------------------------------

;; The denominator every captured amount is exact in.
(define unit-per-em 720)

;; The largest amount any matrix cell states. One em; nothing in Appendices B
;; through E exceeds it.
(define max-amount unit-per-em)

;; How many priority stages a ladder may hold. Section 3.8.3's reduction ladder
;; runs to six and 3.8.4's expansion ladder to four; the bound is deliberately
;; loose and exists to catch a misread ordinal, not to encode the ladders.
(define max-stage 9)

;; A character class as a number. 0 is the line edge -- `line-head` on the row axis
;; of Tables 1 and 3 through 5, `line-end` on their column axis. A character class
;; is 1 through 30.
;;
;; Section 3.9.2 closes the set at thirty, but only twenty-eight of them have
;; adjacency behavior: cl-17 (a ruby annotation) and cl-18 (a warichu) are
;; structures rather than neighbors, so Appendix A enumerates them and no matrix
;; axis carries them. The two predicates below keep that distinction, because
;; reading cl-17 out of Appendix A is correct and reading it off a matrix axis is a
;; corrupt transcription.
(define (class? value)
  (and (exact-integer? value) (<= 1 value 30)))

(define (has-adjacency? value)
  (and (class? value) (not (= value 17)) (not (= value 18))))

(define (axis-class? value)
  (or (eqv? value 0) (has-adjacency? value)))

;; `cl-07`, `line-head` or `line-end` as a class number.
(define (class-of-label label)
  (cond
    [(or (string=? label "line-head") (string=? label "line-end")) 0]
    [(and (= (string-length label) 5) (string=? (substring label 0 3) "cl-"))
     (define value (string->number (substring label 3 5) 10))
     (cond
       [(not (exact-integer? value)) (fail "`~a` is not a class label" label)]
       [(class? value) value]
       [else (fail "`~a` names class ~a, and 3.9.2 closes the set at 30" label value)])]
    [else (fail "`~a` is not a class label" label)]))

;; The inverses, on the row axis and on the column axis.
(define (row-label value)
  (if (eqv? value 0) "line-head" (format "cl-~a" (pad2 value))))

(define (column-label value)
  (if (eqv? value 0) "line-end" (format "cl-~a" (pad2 value))))

(define (pad2 value)
  (if (< value 10) (format "0~a" value) (number->string value)))

;; One matrix coordinate as a single integer, so a pair can key a hash without
;; allocating. Both components are below 64.
(define (coordinate before after)
  (+ (* before 64) after))

;; A legend amount in 1/720 em.
;;
;; The legends write amounts as fractions of an em -- 1/4, 1/2, 3/4, 1/8 -- and 0
;; for solid. A fraction that is not exact in 1/720 em is a violation of
;; `amounts-are-multiples-of-the-unit` and is refused.
(define (amount-of-token token)
  (define (whole text)
    (define value (string->number text 10))
    (unless (exact-integer? value)
      (fail "`~a` is not an integer" text))
    value)
  (define slash (find-char token #\/))
  (define value
    (if (not slash)
        (* (whole token) unit-per-em)
        (let ([numerator (whole (substring token 0 slash))]
              [denominator (whole (substring token (add1 slash)))])
          (when (<= denominator 0)
            (fail "`~a` divides by ~a" token denominator))
          (define scaled (* numerator unit-per-em))
          (unless (zero? (remainder scaled denominator))
            (fail "`~a` is not exactly representable in 1/~a em" token unit-per-em))
          (quotient scaled denominator))))
  (unless (<= 0 value max-amount)
    (fail "`~a` is ~a/~a em, outside [0, 1] em" token value unit-per-em))
  value)

(define (find-char text character)
  (let search ([index 0])
    (cond
      [(>= index (string-length text)) #f]
      [(char=? (string-ref text index) character) index]
      [else (search (add1 index))])))

;; ----------------------------------------------------------------------------
;; Matrix cells
;; ----------------------------------------------------------------------------

;; One amount of a Table 1 cell. `side` is 'before or 'after and names whose
;; neighbor's em the amount was taken from; `hang?` is Appendix B.1's annotation
;; that ruby may extend over that space.
(struct term (amount side hang?) #:transparent)

;; A cell is one of:
;;
;;   'blank                              an empty cell
;;   'prohibited                         the legend's `x`: the adjacency is prohibited
;;   'ruby-hang                          ruby may extend over the character itself
;;   'residual                           3.8.4 step (d), Table 6 only
;;   (no-break levels)                   `not`, or `not 3,4`; '() means every C.3 level
;;   (spacing terms)                     Table 1
;;   (rigid amount stage)                `1/4 stage 3`; `stage` is #f where none is stated
;;   (movable amount limit two-valued? stage)
;;
;; The vocabulary is the one the six legends publish, in the fraction notation
;; rather than either language's words. A hyphen, an en dash and an em dash are the
;; same separator; `=` rather than `-` is 3.1.9's two-valued form, the amount *or*
;; the limit. A token outside this vocabulary is refused rather than ignored.
(struct no-break (levels) #:transparent)
(struct spacing (terms) #:transparent)
(struct rigid (amount stage) #:transparent)
(struct movable (amount limit two-valued? stage) #:transparent)

;; The words of `text`, dropping runs of spaces.
(define (words text)
  (string-split text " " #:trim? #t))

;; The `stage N` suffix and the head it qualifies.
(define (split-stage token)
  (define pieces (words token))
  (define count (length pieces))
  (if (< count 2)
      (values token #f)
      (let ([penultimate (list-ref pieces (- count 2))] [last-piece (list-ref pieces (sub1 count))])
        (if (string=? penultimate "stage")
            (let ([ordinal (string->number last-piece 10)])
              (unless (and (exact-integer? ordinal) (<= 1 ordinal max-stage))
                (fail "`~a` names stage `~a`" token last-piece))
              (values (string-join (take pieces (- count 2)) " ") ordinal))
            (values token #f)))))

;; The offset and width of the first amount/limit separator in `head`, and whether
;; it is the two-valued `=`. U+2013 EN DASH and U+2014 EM DASH are used
;; interchangeably with the hyphen in the legends.
(define (find-separator head)
  (let search ([index 0])
    (cond
      [(>= index (string-length head)) #f]
      [else
       (define character (string-ref head index))
       (cond
         [(char=? character #\-) (list index 1 #f)]
         [(char=? character #\=) (list index 1 #t)]
         [(or (char=? character #\u2013) (char=? character #\u2014)) (list index 1 #f)]
         [else (search (add1 index))])])))

;; One Table 1 term: `1/4 be`, `1/2 af hang`.
(define (parse-term piece)
  (define pieces (words piece))
  (when (< (length pieces) 2)
    (fail "`~a` is not an amount and a side" piece))
  (define amount (amount-of-token (car pieces)))
  (define side
    (case (cadr pieces)
      [("be") 'before]
      [("af") 'after]
      [else (fail "`~a` names the side `~a`, and the legend writes `be` or `af`" piece (cadr pieces))]))
  (define rest (cddr pieces))
  (define hang?
    (cond
      [(null? rest) #f]
      [(and (= (length rest) 1) (string=? (car rest) "hang")) #t]
      [else (fail "`~a` carries `~a`, which the legend does not write" piece (string-join rest " "))]))
  (term amount side hang?))

;; One legend token as a cell.
(define (parse-cell token)
  (define trimmed (string-trim token))
  (cond
    [(or (string=? trimmed "") (string=? trimmed "blank")) 'blank]
    ;; U+00D7 and not the letter x: an ASCII stand-in in a transcription is a
    ;; transcription defect, and reading it as the legend's mark would hide one.
    [(string=? trimmed "×") 'prohibited]
    [(string=? trimmed "ruby hang") 'ruby-hang]
    [(string=? trimmed "residual") 'residual]
    [(string=? trimmed "not") (no-break '())]
    [(string-prefix? trimmed "not ")
     (no-break (for/list ([piece (in-list (string-split (substring trimmed 4) "," #:trim? #t))])
                 (define level (string->number (string-trim piece) 10))
                 (unless (and (exact-integer? level) (<= 1 level 4))
                   (fail "`~a` names the C.3 level `~a`" trimmed piece))
                 level))]
    ;; Table 1: one or more `<amount> be|af [hang]`, joined by `+`.
    [(regexp-match? #rx"(^| )(be|af)( |$)" trimmed)
     (spacing (for/list ([piece (in-list (string-split trimmed "+" #:trim? #t))])
                (parse-term piece)))]
    [else
     (define-values (head stage) (split-stage trimmed))
     (define separator (find-separator head))
     (cond
       [separator
        (define at (car separator))
        (define width (cadr separator))
        (define two-valued? (caddr separator))
        (unless stage
          (fail "`~a` states a limit and no stage" trimmed))
        (movable (amount-of-token (string-trim (substring head 0 at)))
                 (amount-of-token (string-trim (substring head (+ at width))))
                 two-valued?
                 stage)]
       [else (rigid (amount-of-token (string-trim head)) stage)])]))

;; ----------------------------------------------------------------------------
;; The six matrices
;; ----------------------------------------------------------------------------

;; `row-axis` and `column-axis` are vectors of class numbers in file order with
;; duplicates removed; `cells` maps a `coordinate` to a cell; `notes` maps the same
;; coordinate to the Appendix note citation the transcription recorded, or "".
(struct matrix (number row-axis column-axis cells notes) #:transparent)

(define (parse-matrix number text)
  (define file (tsv-parse text))
  (define table-column (tsv-column file "table"))
  (define before-column (tsv-column file "before"))
  (define after-column (tsv-column file "after"))
  (define token-column (tsv-column file "token"))
  (define note-column (tsv-column file "note"))
  (define cells (make-hash))
  (define notes (make-hash))
  (define row-axis '())
  (define column-axis '())
  (for ([row (in-list (tsv-rows file))])
    (define stated (string->number (string-trim (tsv-field row table-column)) 10))
    (unless (eqv? stated number)
      (fail "table~a.tsv carries a row of table ~a" number stated))
    (define before (class-of-label (string-trim (tsv-field row before-column))))
    (define after (class-of-label (string-trim (tsv-field row after-column))))
    (define key (coordinate before after))
    (when (hash-has-key? cells key)
      (fail "Table ~a states (~a, ~a) twice" number (row-label before) (column-label after)))
    (hash-set! cells key (parse-cell (tsv-field row token-column)))
    (hash-set! notes key (string-trim (tsv-field row note-column)))
    (unless (memv before row-axis)
      (set! row-axis (cons before row-axis)))
    (unless (memv after column-axis)
      (set! column-axis (cons after column-axis))))
  (matrix number
          (list->vector (sort (reverse row-axis) <))
          (list->vector (sort (reverse column-axis) <))
          cells
          notes))

;; The cell at one coordinate, or #f where the matrix does not state that pair.
(define (cell-at table before after)
  (hash-ref (matrix-cells table) (coordinate before after) #f))

;; Whether `table` states the pair at all.
(define (states? table before after)
  (hash-has-key? (matrix-cells table) (coordinate before after)))

;; ----------------------------------------------------------------------------
;; Appendix A and the derived Unicode tables
;; ----------------------------------------------------------------------------

;; One row of Appendix A: the class that lists the key, the key as a list of
;; Unicode scalars, and the Remarks cell in both locales.
(struct listing (class key remark-en remark-ja) #:transparent)

;; A hexadecimal scalar sequence such as `304B 309A`.
(define (key-of-field text)
  (define pieces (string-split (string-trim text) " " #:trim? #t))
  (when (null? pieces)
    (fail "an Appendix A row has an empty key"))
  (for/list ([piece (in-list pieces)])
    (define scalar (string->number piece 16))
    (unless (and (exact-integer? scalar)
                 (<= 0 scalar #x10FFFF)
                 (not (<= #xD800 scalar #xDFFF)))
      (fail "`~a` is not a Unicode scalar" piece))
    scalar))

(define appendix-a
  (let* ([file (tsv-parse data:appendix-a)]
         [class-column (tsv-column file "class")]
         [key-column (tsv-column file "key")]
         [en-column (tsv-column file "remark-en")]
         [ja-column (tsv-column file "remark-ja")])
    (for/list ([row (in-list (tsv-rows file))])
      (listing (class-of-label (string-trim (tsv-field row class-column)))
               (key-of-field (tsv-field row key-column))
               (tsv-field row en-column)
               (tsv-field row ja-column)))))

;; The distinct (class, key) listings. One pair is stated twice -- `cl-19 216B`, a
;; defect of the published document that `spec/derived/defects.tsv` records rather
;; than resolves -- so this is one shorter than the row count.
(define appendix-a-listings
  (let ([table (make-hash)])
    (for ([row (in-list appendix-a)])
      (hash-set! table (cons (listing-class row) (listing-key row)) row))
    table))

;; The distinct keys, each with every class that lists it.
(define appendix-a-keys
  (let ([table (make-hash)])
    (for ([row (in-list appendix-a)])
      (hash-update! table (listing-key row) (lambda (classes) (cons (listing-class row) classes)) '()))
    table))

;; The distinct (English, Japanese) Remarks pairs, in first-seen order. Fourteen of
;; them carry every qualification Appendix A states about a listing.
(define appendix-a-remarks
  (let ([seen (make-hash)])
    (for/list ([row (in-list appendix-a)]
               #:unless (hash-has-key? seen (cons (listing-remark-en row) (listing-remark-ja row))))
      (define pair (cons (listing-remark-en row) (listing-remark-ja row)))
      (hash-set! seen pair #t)
      pair)))

;; A field holding exactly one scalar, such as a folding source or a range end.
(define (scalar-of-field text)
  (define key (key-of-field text))
  (unless (null? (cdr key))
    (fail "`~a` names ~a scalars where one was expected" (string-trim text) (length key)))
  (car key))

;; The Wide and Narrow compatibility decompositions.
(struct fold (source target frame) #:transparent)

(define folding
  (let* ([file (tsv-parse data:folding)]
         [source-column (tsv-column file "source")]
         [target-column (tsv-column file "target")]
         [frame-column (tsv-column file "frame")])
    (for/list ([row (in-list (tsv-rows file))])
      (fold (scalar-of-field (tsv-field row source-column))
            (scalar-of-field (tsv-field row target-column))
            (string-trim (tsv-field row frame-column))))))

;; A closed range of scalars.
(struct range-entry (first last) #:transparent)

(define ideographs
  (let* ([file (tsv-parse data:ideographs)]
         [first-column (tsv-column file "first")]
         [last-column (tsv-column file "last")])
    (for/list ([row (in-list (tsv-rows file))])
      (range-entry (scalar-of-field (tsv-field row first-column))
                   (scalar-of-field (tsv-field row last-column))))))

(struct script-range (script first last) #:transparent)

(define scripts
  (let* ([file (tsv-parse data:scripts)]
         [script-column (tsv-column file "script")]
         [first-column (tsv-column file "first")]
         [last-column (tsv-column file "last")])
    (for/list ([row (in-list (tsv-rows file))])
      (script-range (string-trim (tsv-field row script-column))
                    (scalar-of-field (tsv-field row first-column))
                    (scalar-of-field (tsv-field row last-column))))))

(struct class-entry (class name-en name-ja enumeration) #:transparent)

(define classes
  (let* ([file (tsv-parse data:classes)]
         [class-column (tsv-column file "class")]
         [en-column (tsv-column file "name_en")]
         [ja-column (tsv-column file "name_ja")]
         [enumeration-column (tsv-column file "enumeration")])
    (for/list ([row (in-list (tsv-rows file))])
      (class-entry (class-of-label (string-trim (tsv-field row class-column)))
                   (tsv-field row en-column)
                   (tsv-field row ja-column)
                   (tsv-field row enumeration-column)))))

;; Every place JLReq permits more than one answer.
;;
;; Layer M0 keeps the rows as text: the Style resolution these encode is M1's work,
;; and a half-built decoder here would be a guess with a type.
(define questions (tsv-parse data:questions))

;; The six matrices, indexed by their own number.
(define matrices
  (vector (parse-matrix 1 data:table1)
          (parse-matrix 2 data:table2)
          (parse-matrix 3 data:table3)
          (parse-matrix 4 data:table4)
          (parse-matrix 5 data:table5)
          (parse-matrix 6 data:table6)))

(define (matrix-of number)
  (unless (<= 1 number 6)
    (fail "there is no Table ~a" number))
  (vector-ref matrices (sub1 number)))

;; ----------------------------------------------------------------------------
;; The startup census
;; ----------------------------------------------------------------------------

(define (expect what expected actual)
  (unless (equal? expected actual)
    (fail "~a: the specification has ~a, this build read ~a" what expected actual)))

;; Check that what was built is the size and shape `spec/` states.
;;
;; Called from main.rkt before the first request is read. A failure is a build
;; fault, not a request fault, so the engine exits 2 without having answered
;; anything.
(define (self-check)
  ;; Appendix A.
  (expect "Appendix A rows" 1687 (length appendix-a))
  (expect "Appendix A listings" 1686 (hash-count appendix-a-listings))
  (expect "Appendix A distinct keys" 1133 (hash-count appendix-a-keys))
  (expect "Appendix A distinct Remarks pairs" 14 (length appendix-a-remarks))
  (for ([row (in-list appendix-a)])
    (unless (class? (listing-class row))
      (fail "Appendix A lists class ~a, which is not a character class" (listing-class row)))
    (when (null? (listing-key row))
      (fail "an Appendix A row has an empty key")))

  ;; The derived Unicode tables.
  (expect "folding entries" 226 (length folding))
  (expect "Unified_Ideograph ranges" 16 (length ideographs))
  (expect "Hiragana and Katakana ranges" 22 (length scripts))

  ;; The class roster and the Style questions.
  (expect "character classes" 30 (length classes))
  (expect "Style questions" 22 (tsv-row-count questions))

  ;; The six matrices.
  (for ([table (in-vector matrices)])
    (define number (matrix-number table))
    (define axis (if (or (= number 2) (= number 6)) 28 29))
    (expect (format "Table ~a cells" number) (* axis axis) (hash-count (matrix-cells table)))
    (expect (format "Table ~a row axis" number) axis (vector-length (matrix-row-axis table)))
    (expect (format "Table ~a column axis" number) axis (vector-length (matrix-column-axis table)))
    (for ([value (in-vector (matrix-row-axis table))])
      (unless (axis-class? value)
        (fail "Table ~a has a row axis entry of ~a" number value)))
    (for ([value (in-vector (matrix-column-axis table))])
      (unless (axis-class? value)
        (fail "Table ~a has a column axis entry of ~a" number value)))
    ;; Tables 2 and 6 carry no line-edge axis.
    (define edge-expected (and (not (= number 2)) (not (= number 6))))
    (define (has-edge? axis-vector)
      (for/or ([value (in-vector axis-vector)]) (eqv? value 0)))
    (unless (and (eq? (has-edge? (matrix-row-axis table)) edge-expected)
                 (eq? (has-edge? (matrix-column-axis table)) edge-expected))
      (fail "Table ~a disagrees with `line-edge-axes-only-where-they-exist`" number))
    ;; Every axis pair is stated exactly once, so the matrix is complete.
    (for* ([before (in-vector (matrix-row-axis table))]
           [after (in-vector (matrix-column-axis table))])
      (unless (states? table before after)
        (fail "Table ~a has no cell at (~a, ~a)" number (row-label before) (column-label after))))
    ;; Every amount is in [0, 1] em; every stage ordinal is in its ladder.
    (for ([found (in-hash-values (matrix-cells table))])
      (check-amounts number found)))

  ;; `residual` is Table 6's alone, `not` is Table 2's alone, and a Table 1
  ;; spacing token is Table 1's alone.
  (for ([table (in-vector matrices)])
    (define number (matrix-number table))
    (for ([found (in-hash-values (matrix-cells table))])
      (cond
        [(and (eq? found 'residual) (not (= number 6)))
         (fail "Table ~a states `residual`, which is 3.8.4 step (d)'s and Table 6's" number)]
        [(and (no-break? found) (not (= number 2)))
         (fail "Table ~a states `not`, which is Table 2's" number)]
        [(and (spacing? found) (not (= number 1)))
         (fail "Table ~a states a Table 1 spacing token" number)]
        [else (void)]))))

(define (check-amounts number found)
  (define (check-amount value)
    (unless (<= 0 value max-amount)
      (fail "Table ~a states ~a/~a em" number value unit-per-em)))
  (define (check-stage stage)
    (when stage
      (unless (<= 1 stage max-stage)
        (fail "Table ~a states stage ~a" number stage))))
  (cond
    [(spacing? found) (for ([one (in-list (spacing-terms found))]) (check-amount (term-amount one)))]
    [(rigid? found)
     (check-amount (rigid-amount found))
     (check-stage (rigid-stage found))]
    [(movable? found)
     (check-amount (movable-amount found))
     (check-amount (movable-limit found))
     (check-stage (movable-stage found))]
    [else (void)]))
