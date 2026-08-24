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
         (struct-out attachment)
         (struct-out contribution)
         boundary-contributions
         head-contributions
         end-contributions
         total-of
         prohibited?
         stated-cell
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
;;
;; `structure` names the §3.4 or §3.7 structure the item is part of, where it is
;; part of one: `(formula display?)` for §3.7.4, `(warichu index)` for the text of
;; an inline cutting note. It is #f for everything a line sets straight along
;; itself.
(struct item (index start end advance size frame role class transform kind members
              complex run separation tail attachments structure)
  #:transparent)

;; One cluster of a grouped item.
;;
;; Most items are one cluster and carry one of these. A construct that sets several
;; clusters as one thing on the line -- a tate-chu-yoko run, a base character group,
;; a warichu block -- carries one per cluster, and `compose.rkt` gives each its own
;; place inside the item's own box. `block` is the offset from the item's own block
;; origin, which is zero for everything a line sets straight along it.
(struct piece (index start end advance size frame transform inline block writing-mode) #:transparent)

;; One annotation, waiting for its base to be placed.
;;
;; `anchor` is the item its position is measured from. `span` is how many items it
;; is set across: 0 means `offset` is the whole answer, which is what a ruby reading
;; carries, and a positive span means the annotation is CENTERED over that many
;; items and the offset is computed once the line knows how wide they came out.
;; §3.3.9 and §3.7.1 both need the second, because both center on "the base
;; characters" and what the line gave those characters includes the space beside
;; them.
;;
;; `symbol` is the repeated mark §3.3.9 sets and #f for an annotation that carries
;; its own text; a symbol takes no advance of its own.
(struct attachment (construct anchor span whole offset start end advance size symbol) #:transparent)

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

;; Whether the item is the last character of a §3.4 structure, and whether it is the
;; first.
;;
;; The BRACKET is the structure's own edge. A bare note ends with a character of its
;; own text, which is the last character of a SUBLINE -- inside the block, where the
;; block has already stopped reading Table 1 -- and the space beside the block is
;; still the neighbor's own.
(define (structure-edge? one last?)
  (let ([found (item-structure one)])
    (and found
         (eq? (car found) 'warichu)
         (= (length found) 5)
         (not (list-ref found 2))
         (list-ref found (if last? 3 4)))))

(define (ends-structure? one) (structure-edge? one #t))
(define (starts-structure? one) (structure-edge? one #f))

;; Drop the terms `owner` contributes.
(define (without terms owner)
  (filter (lambda (one) (not (eq? (contribution-owner one) owner))) terms))

;; ----------------------------------------------------------------------------
;; Boundaries
;; ----------------------------------------------------------------------------

;; The space between two occurrences of one line.
;; §3.6: a tab sign is the space between two characters and not a character.
(define (tab? one)
  (eq? (item-kind one) 'tab))

;; The cell Table 1 states between two occurrences, with no section's own withdrawal
;; and no rule about what one of them is. §3.7.3's own room is measured in this: a
;; jidori is a statement about how many CELLS the run occupies, so what it has to
;; put in them is the width the transcription gives the run and not the width some
;; other section has since taken out of it.
(define (stated-cell before after)
  (total-of (cell-contributions (item-class before) (item-class after)
                                (item-em before) (item-em after))))

(define (boundary-contributions before after writing-mode style [literal? #f])
  (define stated
    (cond
      ;; What the transcription states, before any section has withdrawn it. The tab
      ;; clause below is one of the withdrawals and is under this one for the same
      ;; reason §3.2.5's cl-30 cells are: Appendices D and E measure their room in
      ;; the cell as transcribed, so a line that has to give space back gives back
      ;; the quarter em beside a middle dot even where the sign in front of it had
      ;; taken that quarter em up.
      [literal?
       (cell-contributions (item-class before) (item-class after) (item-em before) (item-em after))]
      ;; The space after a tab sign is the sign: §3.6.2 puts the string at the stop
      ;; and the sign is what takes up the distance, so there is nothing left at
      ;; that boundary for Table 1 to state. The boundary BEFORE a sign is an
      ;; ordinary one -- what stands there is the space the character before the
      ;; sign takes, and the sign begins after it.
      [(tab? before) '()]
      [else (or (formula-terms before after) (table-one-terms before after))]))
  (define withdrawn
    (let* ([step-one (if (withdraws-own-space? before writing-mode) (without stated 'before) stated)]
           [step-two (if (withdraws-own-space? after writing-mode) (without step-one 'after) step-one)]
           ;; §3.4: a stacked structure's own edge characters carry no space, and
           ;; the structure does. The brackets that open and close an inline cutting
           ;; note are characters of the STRUCTURE rather than of the line, so an
           ;; amount Table 1 states in one of THEIR ems stands beside the whole block
           ;; and is no part of the bracket's own. What the character outside the
           ;; structure owns is still its own and still stands, which is the quarter
           ;; em a middle dot takes after a note.
           [step-three (if (ends-structure? before) (without step-two 'before) step-two)]
           [step-four (if (starts-structure? after) (without step-three 'after) step-three)])
      step-four))
  (append withdrawn
          (sentence-terminator-terms before after)
          (sentence-medial-terms before after style)))

;; §3.7.4's own answer at a boundary one of whose sides is a math symbol (cl-17) or
;; a math operator (cl-18), or #f where the section says nothing about the
;; coordinate.
;;
;; No matrix carries a cl-17 or a cl-18 axis, so Table 1 answers nothing here and
;; the section is the whole of it. Inside an ordinary line the boundary is solid;
;; on an independent line a math SYMBOL opens a quarter em of its own em on each
;; side and a math OPERATOR stays solid. The em is the symbol's own, because
;; §3.7.4 is a statement about how the symbol is set.
(define (formula-terms before after)
  (define (math? one) (memv (item-class one) '(17 18)))
  (define (beside? one) (memv (item-class one) '(21 24 27)))
  (define (display? one)
    (let ([found (item-structure one)])
      (and found (eq? (car found) 'formula) (cadr found))))
  (cond
    [(and (math? before) (beside? after))
     (if (and (display? before) (eqv? (item-class before) 17))
         (list (contribution (scale 180 (item-em before)) 'before #f 180))
         '())]
    [(and (beside? before) (math? after))
     (if (and (display? after) (eqv? (item-class after) 17))
         (list (contribution (scale 180 (item-em after)) 'after #f 180))
         '())]
    [(or (math? before) (math? after)) '()]
    [else #f]))

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
(define (end-contributions last-item style writing-mode [next #f])
  (define stated
    (cond
      ;; §3.2.5: "when tate-chu-yoko is set before an opening bracket, half em
      ;; spacing is added". The section is describing how the RUN is set -- it is
      ;; one object on the line however many characters it holds -- rather than
      ;; what happens at a boundary, so the half em is there because an opening
      ;; bracket follows the run in the text and stays there when the line ends
      ;; between the two. Every other amount at a line end is Table 1's `line-end`
      ;; column, which is what a boundary looks like once one of its neighbors is
      ;; gone.
      [(and next (= (item-class last-item) 30) (= (item-class next) 1))
       (cell-contributions 30 1 (item-em last-item) (item-em next))]
      [(withdraws-own-space? last-item writing-mode)
       (without (cell-contributions (item-class last-item) line-edge (item-em last-item) #f) 'before)]
      [else (cell-contributions (item-class last-item) line-edge (item-em last-item) #f)]))
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
