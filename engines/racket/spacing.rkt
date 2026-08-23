#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; Appendix B: how much space stands at one boundary.
;;
;; A boundary has two neighbors and at most two owners. Table 1 states the amount
;; as one or two terms, and each term names whose em the fraction is a fraction of —
;; `be` for the preceding character's, `af` for the trailing one's (§B.1). That is
;; the whole reason the legend has two words for a half em: a line may hold two
;; sizes, and "half an em" without an owner is not a number.
;;
;; Each term is therefore kept as an amount *and* an owner all the way through, and
;; the sections that withdraw a space withdraw one owner's term rather than the
;; boundary's total. §3.1.3 is the clearest case: a middle dot standing inside a
;; unit symbol, a grouped numeral or a formula carries no space of its own, and the
;; neighbor on the other side of that same boundary still carries its own — a full
;; stop before such a dot keeps its half em and loses only the dot's quarter
;; (§B.2 note 12).
;;
;; The two line edges are coordinates like any other: Table 1's `line-head` row and
;; `line-end` column, read against the class of the first and the last character of
;; the line. A `be` term in the `line-end` column is the last character's own em,
;; which is why a closing bracket ending a line takes a half em of its own size
;; after it (§3.1.9, §B.2 note 2).
;;
;; What is *not* here: the amount a space is reduced or expanded to. That is
;; Appendices D and E, and `adjust.rkt` reads them at the same coordinate.

(require racket/list
         (prefix-in tables: "tables.rkt")
         "classes.rkt"
         "style.rkt"
         "model.rkt"
         "arith.rkt")

(provide (struct-out item)
         (struct-out piece)
         (struct-out contribution)
         boundary-contributions
         head-contributions
         end-contributions
         total-of
         prohibited?
         item-em)

;; One occurrence on the line: the cluster it came from, and everything the layout
;; needs to know about it.
;;
;; `class` is §3.9.2's answer for this occurrence; `advance` is what the caller
;; shaped it to, before any space Table 1 adds beside it.
;;
;; The last four are what a construct does to the item. `complex` names the base
;; character group it belongs to, which is what §B.2 notes 9 through 11 and §E.2
;; notes 5 through 7 mean by "the same complex": no space stands between two items
;; of one, and none opens there. `run` is the narrower unit §C.2 notes 6 through 8
;; refuse a line break inside. `separation` is the space §3.3 forced in BEFORE this
;; item because a reading had nowhere else to go, `tail` the same at the paragraph's
;; own end -- the one boundary that is not before any item -- and `attachments` is
;; the reading itself, positioned from this item's own start.
(struct item (index start end advance size frame role class transform kind members
              complex run separation tail attachments)
  #:transparent)

;; One cluster of a grouped item.
;;
;; Most items are one cluster and carry one of these. A construct that sets several
;; clusters as one thing on the line -- a tate-chu-yoko run, a base character group,
;; a warichu block -- carries one per cluster, and `compose.rkt` gives each its own
;; place inside the item's own box. `block` is the offset from the item's own block
;; origin, which is zero for everything a line sets straight along it.
(struct piece (index start end advance size frame transform block writing-mode) #:transparent)

;; One term of a boundary: an amount in the caller's unit and whose em it came
;; from. `hang?` is §B.1's annotation that a ruby reading may extend over it, which
;; M6 reads and M1 only carries. `units` is the same amount in the 1/720 em the six
;; matrices are transcribed in, kept because Appendix D states its floors in that
;; unit and against this term rather than against the boundary's total.
(struct contribution (amount owner hang? units) #:transparent)

;; The em a term is measured against.
(define (item-em one)
  (extent-inline (item-size one)))

;; `amount` is in 1/720 em; bring it into the caller's unit against `em`.
;;
;; Every amount the six matrices state is an exact multiple of 1/720 em and every em
;; the suite states is a multiple of one seven-hundred-and-twentieth of itself, so this division is exact in
;; practice; `div-trunc` is what it does when a caller's em is not.
(define (scale amount em)
  (div-trunc (chk* amount em) tables:unit-per-em))

;; ----------------------------------------------------------------------------
;; Table 1
;; ----------------------------------------------------------------------------

(define table1 (tables:matrix-of 1))

;; Whether Table 1 marks the coordinate as one that does not occur.
(define (prohibited? before after)
  (eq? (tables:cell-at table1 before after) 'prohibited))

;; The terms Table 1 states at (`before`, `after`), as amounts against the two ems.
;;
;; `before-em` and `after-em` are the ems the two owners are measured in; either may
;; be #f where that side is a line edge, in which case no term of that owner exists
;; to measure (Table 1's edge row and column state `be` terms only, and only in the
;; `line-end` column).
(define (cell-contributions before after before-em after-em)
  (define found (tables:cell-at table1 before after))
  (cond
    [(tables:spacing? found)
     (for/list ([one (in-list (tables:spacing-terms found))])
       (define owner (tables:term-side one))
       (define em (if (eq? owner 'before) before-em after-em))
       (unless em
         (fail-input "Table 1 states a `~a` term at a line edge" owner))
       (contribution (scale (tables:term-amount one) em) owner (tables:term-hang? one)
                     (tables:term-amount one)))]
    [else '()]))

;; ----------------------------------------------------------------------------
;; The withdrawals
;; ----------------------------------------------------------------------------

;; Whether this occurrence carries no space of its own, by §3.1.3.
;;
;; Two of the section's three cases are vertical composition only — an IDEOGRAPHIC
;; COMMA used as a decimal separator, a KATAKANA MIDDLE DOT used as a decimal point
;; — and the third, a middle dot inside a unit symbol, a grouped numeral or a
;; formula, is stated in vertical composition by the English rendering and in
;; horizontal composition by the Japanese one. The reading taken for the third is
;; the union of the two locales, which is also what §B.2 note 12 states with no mode
;; at all; the first two are read as the sections' own single mode.
(define (withdraws-own-space? one writing-mode)
  (define value (item-class one))
  (define role (item-role one))
  (or (middle-dot-in-construct? value role)
      (and (eq? writing-mode 'vertical-rl)
           (or (and (= value 5) (eq? role 'decimal-point))
               (and (= value 7) (eq? role 'digit-group-separator))))))

;; Drop the terms `owner` contributes.
(define (without terms owner)
  (filter (lambda (one) (not (eq? (contribution-owner one) owner))) terms))

;; ----------------------------------------------------------------------------
;; Boundaries
;; ----------------------------------------------------------------------------

;; The space between two occurrences of one line.
(define (boundary-contributions before after writing-mode style)
  (define stated (table-one-terms before after))
  (define withdrawn
    (let* ([step-one (if (withdraws-own-space? before writing-mode) (without stated 'before) stated)]
           [step-two (if (withdraws-own-space? after writing-mode) (without step-one 'after) step-one)])
      step-two))
  (append withdrawn
          (sentence-terminator-terms before after)
          (sentence-medial-terms before after style)))

;; What Table 1 states at one interior boundary, with §3.2.5's own override.
;;
;; §3.2.5 states four amounts beside a tate-chu-yoko run (cl-30) -- a half em after
;; a comma (cl-07), a closing bracket (cl-02) or a mid-line full stop (cl-06), a
;; half em before an opening bracket (cl-01), and solid otherwise -- and then says
;; the details "are described as a complete table in §B". Table 1's cl-30 row and
;; column state those four AND SIX MORE: a quarter em against a middle dot (cl-05)
;; in both directions, and against cl-21, cl-24, cl-25 and cl-27 in both. The prose
;; wins over the sentence that points at the table, so the four stand and the six do
;; not. Appendix D and Appendix E are read at face value at the same coordinates,
;; which is why this override is here rather than in the matrix.
(define (table-one-terms before after)
  (define stated
    (cell-contributions (item-class before) (item-class after) (item-em before) (item-em after)))
  (define run-before? (= (item-class before) 30))
  (define run-after? (= (item-class after) 30))
  (cond
    [(and (not run-before?) (not run-after?)) stated]
    [(and run-before? (= (item-class after) 1)) stated]
    [(and run-after? (memv (item-class before) '(2 6 7))) stated]
    [else '()]))

;; §3.1.6 and §C.2 note 4: a dividing punctuation mark (cl-04) that ends a sentence
;; takes one em after it. The em is the mark's own, which is the character §3.1.6 is
;; describing when it calls the mark full-width and then states what follows it.
;;
;; Like the sentence-medial Note below, this reaches only a coordinate Table 1
;; states nothing at. Where Table 1 already carries a term -- the half em before an
;; opening bracket, the quarter em before a middle dot -- the cell has an answer
;; from a different sentence, and a sentence-final mark does not make that sentence
;; stop applying.
(define (sentence-terminator-terms before after)
  (cond
    [(not (and (= (item-class before) 4) (eq? (item-role before) 'sentence-terminator))) '()]
    [(not (null? (cell-contributions (item-class before) (item-class after)
                                     (item-em before) (item-em after))))
     '()]
    [else (list (contribution (scale tables:unit-per-em (item-em before)) 'before #f tables:unit-per-em))]))

;; §3.1.6's third Note: a dividing punctuation mark (cl-04) used in the middle of a
;; sentence takes either no spacing or a quarter em on both sides.
;;
;; `docs/decisions/sentence-medial-dividing-mark.md` settles what the Note leaves
;; open. The quarter em is the *mark's* own em on both sides, and the Note reaches
;; only a coordinate Table 1 states nothing at: where Table 1 already carries a term
;; the cell has an answer, given by a different sentence, and "add no spacing" is not
;; silent about it. A line edge is declined explicitly — the Note's own two
;; positions presuppose a character on each side.
(define (sentence-medial-terms before after style)
  (cond
    [(not (answer-is? style "spacing.sentence_medial_dividing_mark" "quarter-em")) '()]
    [(not (null? (cell-contributions (item-class before) (item-class after) (item-em before) (item-em after))))
     '()]
    [(and (= (item-class after) 4) (eq? (item-role after) 'sentence-medial))
     (list (contribution (scale 180 (item-em after)) 'after #f 180))]
    [(and (= (item-class before) 4) (eq? (item-role before) 'sentence-medial))
     (list (contribution (scale 180 (item-em before)) 'before #f 180))]
    [else '()]))

;; The space between the line head and the line's first occurrence.
;;
;; §3.1.5's three patterns are what `spacing.line_head_opening_bracket` chooses
;; among, and B.2 note 17 states the preferred one: nothing at all. Pattern 2 keeps
;; the conditional half em an opening bracket carries, so the bracket stands half an
;; em in from the head of every line; pattern 3 pulls the first line of a paragraph
;; back by the same half em instead, and leaves a wrapped line flush. Both are
;; measured against the bracket's own em.
(define (head-contributions first-item style first-line? writing-mode)
  (define pattern (answer style "spacing.line_head_opening_bracket"))
  (define stated
    (if (withdraws-own-space? first-item writing-mode)
        (without (cell-contributions line-edge (item-class first-item) #f (item-em first-item)) 'after)
        (cell-contributions line-edge (item-class first-item) #f (item-em first-item))))
  (cond
    [(not (= (item-class first-item) 1)) stated]
    [(string=? pattern "pattern-2") (list (contribution (scale 360 (item-em first-item)) 'after #f 360))]
    [(and (string=? pattern "pattern-3") first-line?)
     (list (contribution (- (scale 360 (item-em first-item))) 'after #f (- 360)))]
    [else stated]))

;; The space between the line's last occurrence and the line end.
(define (end-contributions last-item style writing-mode)
  (define stated
    (if (withdraws-own-space? last-item writing-mode)
        (without (cell-contributions (item-class last-item) line-edge (item-em last-item) #f) 'before)
        (cell-contributions (item-class last-item) line-edge (item-em last-item) #f)))
  (cond
    ;; §3.1.9 and §B.2 notes 2 and 6: the half em a closing bracket, a full stop or
    ;; a comma takes at the line end is what `spacing.line_end_punctuation` and
    ;; `spacing.line_end_full_stop_comma` answer for. `solid` and `jis` withdraw it.
    [(and (= (item-class last-item) 2)
          (answer-is? style "spacing.line_end_punctuation" "solid"))
     '()]
    [(and (or (= (item-class last-item) 6) (= (item-class last-item) 7))
          (answer-is? style "spacing.line_end_full_stop_comma" "jis"))
     '()]
    [else stated]))

;; The amount a list of terms adds up to.
(define (total-of terms)
  (for/fold ([sum 0]) ([one (in-list terms)]) (chk+ sum (contribution-amount one))))
