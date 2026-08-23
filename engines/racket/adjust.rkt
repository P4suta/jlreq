#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; Appendices D and E: how far a boundary may be squeezed and how far it may be
;; opened.
;;
;; Tables 3, 4 and 5 answer the first and Table 6 the second, at the same
;; coordinates Table 1 states the amount at. Neither states an *amount* of its own
;; that a line has to start from: the default is Table 1's, and what these tables
;; add is a floor, a ceiling, and the priority stage the ladder reaches the site at.
;;
;; Whose em
;;
;; Table 1's terms name their owner and these do not (ADR 0021: Table 6 states one
;; cell per class pair with no `be` or `af` column at all). Two different answers
;; follow from that, and they are different because the two tables are asked
;; different questions.
;;
;; A *reduction* floor qualifies an amount that already exists, so it is read as a
;; proportion of that amount: `1/4-1/8` halves whatever the quarter em came to,
;; whichever owner's em it was measured against. That keeps the floor meaningful on
;; a line of two sizes without inventing an owner Appendix D never named, and on a
;; line of one size it is the same number either way.
;;
;; An *expansion* ceiling frequently qualifies an amount that does not exist —
;; Table 6's `0-1/4` sits at coordinates Table 1 leaves solid — so a proportion of
;; nothing is nothing and an em has to be chosen. `expansion-ceiling` takes the
;; boundary's own two neighbors and reads the ceiling against the trailing one's
;; em, which is the em §3.8.4's own "with proportional character size" is measured
;; in for the space that opens *before* a character.
;;
;; Stages
;;
;; §3.8.3's ladder runs to six and §3.8.4's to four, and both tables carry the stage
;; in the cell. One stage is not in either table: §D's own first stage, the Western
;; word space (cl-26), which "is already done" by the time Tables 3 through 5 are
;; read. It is stage 1 of the reduction ladder and stage 1 of the expansion ladder
;; alike, and it is a fact about a *character's advance* rather than about a
;; boundary, so `word-space-floor` and `word-space-ceiling` state it here and
;; `compose.rkt` applies it to the cluster.

(require (prefix-in tables: "tables.rkt") "spacing.rkt" "style.rkt" "arith.rkt")

(provide reduction-of
         expansion-of
         (struct-out reduction)
         (struct-out expansion)
         word-space-floor
         word-space-ceiling
         reduction-table-of)

;; What Appendix D permits at one coordinate.
;;
;; `floor` is the least the amount may become, `stage` the priority the ladder
;; reaches it at, and `two-valued?` §3.1.9's restriction that the amount is the
;; stated one or nothing, with no size in between.
(struct reduction (floor stage two-valued?) #:transparent)

;; What Appendix E permits at one coordinate.
;;
;; `ceiling` is the most the amount may become and `stage` the priority; `residual?`
;; is Table 6's own marker for a site step (d) reaches and the three numbered stages
;; do not.
(struct expansion (ceiling stage residual?) #:transparent)

;; The table `adjustment.reduction_table` selects.
(define (reduction-table-of style)
  (define chosen (answer style "adjustment.reduction_table"))
  (tables:matrix-of (cond
                      [(string=? chosen "table-3") 3]
                      [(string=? chosen "table-4") 4]
                      [else 5])))

(define table6 (tables:matrix-of 6))

;; `amount` scaled by `numerator/denominator`, truncating toward zero.
(define (proportion amount numerator denominator)
  (if (zero? denominator) 0 (div-trunc (chk* amount numerator) denominator)))

;; What Appendix D permits at (`before`, `after`), given the amount Table 1 put
;; there.
;;
;; A cell that states a rigid amount, a blank, or a prohibition offers nothing: the
;; floor is the amount itself. The stage is #f where there is no opportunity, which
;; is how `compose.rkt` tells a site it may reduce from one it may not.
(define (reduction-of table before after terms)
  (define found (tables:cell-at table before after))
  (cond
    [(tables:movable? found)
     (reduction (floor-of terms (- (tables:movable-amount found) (tables:movable-limit found)))
                (tables:movable-stage found)
                (tables:movable-two-valued? found))]
    [else (reduction (total-of terms) #f #f)]))

;; What is left of `terms` once `capacity` 1/720 em of them has been given back.
;;
;; Appendix D states its floors against the *term* a reduction qualifies and not
;; against the boundary's total, and the two are different numbers wherever Table 1
;; states two terms: §D.2 note 2 reduces the middle dot's own quarter em and leaves
;; the full stop's half em standing, and Table 5's own cell at the same coordinate
;; states `1/2-1/4` against a boundary that adds up to three quarters. So the
;; capacity is taken from the trailing term first and from the leading one only when
;; that is not enough -- which is the order the notes take it in -- and each term is
;; scaled against its own owner's em on the way down.
(define (floor-of terms capacity)
  (let walk ([rest (reverse terms)] [left capacity] [sum 0])
    (cond
      [(null? rest) sum]
      [else
       (define one (car rest))
       (define units (contribution-units one))
       (define taken (if (positive? units) (min left units) 0))
       (define kept
         (if (positive? units)
             (div-trunc (chk* (contribution-amount one) (- units taken)) units)
             (contribution-amount one)))
       (walk (cdr rest) (- left taken) (chk+ sum kept))])))

;; What Appendix E permits at (`before`, `after`), against `em`.
;;
;; `blank` and `residual` are both step (d): §E.1 reads a blank cell as "expandable
;; equally with respect to the corresponding character size, only after no other
;; expandable inter-character spacing is left", which is what step (d) does, and
;; `residual` is the transcription's name for the same answer where the published
;; table shades the cell rather than leaving it empty. A prohibited coordinate is
;; neither.
(define (expansion-of before after em)
  (define found (tables:cell-at table6 before after))
  (cond
    [(tables:movable? found)
     ;; A numbered stage opens the site to its own ceiling, and step (d) then
     ;; reaches it again: §E.1 says the fourth step adds space "to equalize the
     ;; spacing of 1st, 2nd, 3rd and 4th steps", so a boundary the second or third
     ;; stage already opened is one of the places the residual is leveled over.
     (expansion (div-trunc (chk* (tables:movable-limit found) em) tables:unit-per-em)
                (tables:movable-stage found)
                #t)]
    ;; The transcription's name for the shaded cell: a site the three numbered
    ;; stages do not reach and step (d) does.
    [(eq? found 'residual) (expansion #f #f #t)]
    ;; A blank cell is not a step (d) site. Table 6 is the whole of where a line may
    ;; be opened, and the 270 cells it leaves empty are the coordinates that offer
    ;; nothing -- an opening bracket before an ideograph among them, which is what
    ;; makes a justified line holding only that boundary come out at its natural
    ;; width rather than at the measure.
    [else (expansion #f #f #f)]))

;; §3.8.3 (a) and §D: a Western word space is reduced to leave a quarter em.
(define (word-space-floor em)
  (div-trunc (chk* 180 em) tables:unit-per-em))

;; §3.8.4 (a) and §E: a Western word space takes up to half an em.
(define (word-space-ceiling em)
  (div-trunc (chk* 360 em) tables:unit-per-em))
