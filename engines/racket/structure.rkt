#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; §3.4, §3.7.2, §3.7.3 and §3.7.4: the structures that are not text along the
;; line.
;;
;; **§3.7.4, formulae.** A math symbol (cl-17) and a math operator (cl-18) are one
;; em wide whatever the caller shaped them to, and the space beside them is stated
;; by §3.7.4 rather than by Table 1 -- which has no cl-17 or cl-18 axis at all,
;; because neither class carries adjacency in any matrix. Two settings, and the
;; section names them: a formula set INSIDE an ordinary line is solid against the
;; grouped numerals, Western characters and ornamented complexes beside it, and one
;; set as an independent line opens a quarter em beside a math SYMBOL and stays
;; solid beside a math OPERATOR. A formula whose own range is the whole paragraph
;; is the independent line; anything shorter is inside one.
;;
;; §3.7.4's two named classes are also the whole of where a formula may break: "a
;; line break in a mathematical formula is done, when possible, at an equals sign
;; (cl-17) ... or at an operator (cl-18)" is read as a rule rather than a
;; preference, so a break with neither beside it is refused. On an independent line
;; the section states a priority as well -- before a math symbol first, before a
;; math operator second -- and that is a preference among the breaks that exist.
;;
;; **§3.7.3, jidori.** A run of characters is set across a declared number of
;; full-em cells. What the run does not fill is shared out over its own internal
;; boundaries, and the run is indivisible: §3.7.3's own subject is the run.
;;
;; **§3.7.2, furawake, and §3.4, warichu.** Both set their text on two or more
;; lines beside the main one. The geometry they share is here: the rows are laid
;; out one after another along the line, the block they make is centered across the
;; main line's own em, and what the structure takes up ALONG the line is its longest
;; row rather than the sum of its characters.
;;
;; Where they differ is the division. A furawake divides where the caller said and
;; as many ways as the caller declared columns. A warichu divides itself, into two
;; rows as near the same length as the characters allow -- and the caller's own
;; break opportunities are not that division but the places the whole note may be
;; cut between two MAIN lines (§3.4.3), each piece then dividing itself again.

(require racket/list
         "arith.rkt"
         "model.rkt"
         "style.rkt"
         "spacing.rkt")

(provide formula-constructs
         display-formula?
         formula-space
         math-class?
         jidori-separations
         rows-of
         row-block-offsets
         balanced-split)

;; ----------------------------------------------------------------------------
;; §3.7.4
;; ----------------------------------------------------------------------------

(define (math-class? value)
  (or (eqv? value 17) (eqv? value 18)))

(define (formula-constructs para)
  (for/list ([one (in-list (paragraph-constructs para))]
             #:when (eq? (construct-kind one) 'formula))
    one))

;; Whether a formula is the independent line §3.7.4 states the second setting for.
;;
;; The protocol has no field saying so, and the fact is in the request all the same:
;; a formula whose own range is the whole of the caller's text is a line of its own,
;; and one with text on either side of it is inside a line.
(define (display-formula? one para)
  (define clusters (paragraph-clusters para))
  (and (positive? (vector-length clusters))
       (<= (construct-start one) (cluster-start (vector-ref clusters 0)))
       (>= (construct-end one) (cluster-end (vector-ref clusters (sub1 (vector-length clusters)))))))

;; The space §3.7.4 states at a boundary one of whose sides is a math symbol or a
;; math operator, as a list of contributions, or #f where the section says nothing
;; about this coordinate and Table 1's own answer stands.
;;
;; The em is the symbol's own: §3.7.4 is a statement about how the symbol is set,
;; and the symbol is the character both of its boundaries have in common.
(define (formula-space before after display?)
  (define quarter 180)
  (define (beside? class) (memv class '(21 24 27)))
  (cond
    [(and (math-class? (item-class before)) (beside? (item-class after)))
     (if (and display? (eqv? (item-class before) 17))
         (list (contribution (scale quarter (item-em before)) 'before #f))
         '())]
    [(and (beside? (item-class before)) (math-class? (item-class after)))
     (if (and display? (eqv? (item-class after) 17))
         (list (contribution (scale quarter (item-em after)) 'after #f))
         '())]
    [(or (math-class? (item-class before)) (math-class? (item-class after))) '()]
    [else #f]))

(define (scale amount em)
  (div-trunc (chk* amount em) 720))

;; ----------------------------------------------------------------------------
;; §3.7.3
;; ----------------------------------------------------------------------------

;; The space a jidori forces in, as a list of `(item-index . amount)`.
;;
;; `bases` is the construct's own items in line order. The cells are full-em cells
;; of the paragraph's own size, and what the run does not fill is shared over its
;; internal boundaries -- the space is between the characters and not around them,
;; because §3.7.3 spreads the run across the cells rather than placing it in one.
(define (jidori-separations cells em bases style)
  (define want (chk* cells em))
  (define have (for/fold ([sum 0]) ([one (in-list bases)]) (chk+ sum (cdr one))))
  (define room (max 0 (chk- want have)))
  (define sites (sub1 (length bases)))
  (cond
    [(or (zero? room) (<= sites 0)) '()]
    [else
     (define share (div-trunc room sites))
     (define left (rem-trunc room sites))
     (define trailing? (answer-is? style "adjustment.remainder" "trailing"))
     (for/list ([one (in-list (cdr bases))] [rank (in-naturals)])
       (define extra
         (if trailing?
             (if (>= rank (- sites left)) 1 0)
             (if (< rank left) 1 0)))
       (cons (car one) (chk+ share extra)))]))

;; ----------------------------------------------------------------------------
;; The shared geometry of a stacked structure
;; ----------------------------------------------------------------------------

;; Split `widths` into `count` rows at the positions `at`, which are indices into
;; the list. Answers a list of lists of indices.
(define (rows-of count size at)
  (let walk ([index 0] [rest at] [row '()] [out '()])
    (cond
      [(>= index size)
       (reverse (cons (reverse row) out))]
      [(and (pair? rest) (= index (car rest)) (pair? row))
       (walk index (cdr rest) '() (cons (reverse row) out))]
      [else (walk (add1 index) rest (cons index row) out)])))

;; Where a stacked structure's own two or more rows stand across the line.
;;
;; The block they make is centered on the main line's own em: a note two half-size
;; rows deep sits inside the line it interrupts, and a furawake two full rows deep
;; hangs equally above and below it. The coordinate a placement reports is the
;; box's own leading edge in the direction the lines progress, so the two writing
;; modes count from opposite ends of the same stack.
(define (row-block-offsets heights gap em vertical?)
  (define total
    (chk+ (for/fold ([sum 0]) ([one (in-list heights)]) (chk+ sum one))
          (chk* gap (max 0 (sub1 (length heights))))))
  (define start (chk- (div-trunc em 2) (div-trunc total 2)))
  (let walk ([rest heights] [at start] [out '()])
    (cond
      [(null? rest) (reverse out)]
      [else
       ;; The two writing modes are mirror images about the line's own origin: a
       ;; horizontal row reports the top of its box and a vertical one the right of
       ;; it, so the same stack read from the other end is the same number negated.
       (walk (cdr rest)
             (chk+ (chk+ at (car rest)) gap)
             (cons (if vertical? (- at) at) out))])))

;; §3.4.2: divide `widths` into two rows "as near the same length as they can be
;; made", with the second no longer than the first where the characters allow it.
;;
;; The answer is how many of them go on the first row. Where two positions balance
;; equally the earlier one is taken, and the preference for a first row at least as
;; long as the second is what settles an odd count.
(define (balanced-split widths)
  (define total (for/fold ([sum 0]) ([one (in-list widths)]) (chk+ sum one)))
  (define count (length widths))
  (cond
    [(<= count 1) count]
    [else
     (let walk ([index 1] [ahead (car widths)] [best 1] [score #f])
       (cond
         [(>= index count) best]
         [else
          (define behind (chk- total ahead))
          ;; A first row shorter than the second is a worse answer than one longer
          ;; by the same amount, which is what "should not be longer" states.
          (define here
            (if (>= ahead behind) (chk- ahead behind) (chk+ (chk* (chk- behind ahead) 2) 1)))
          (define better? (or (not score) (< here score)))
          (walk (add1 index)
                (chk+ ahead (list-ref widths index))
                (if better? index best)
                (if better? here score))]))]))
