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
         (struct-out boundary)
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

;; One internal boundary of a jidori run: the amount Table 1 already states there,
;; the item that stands after it, whether a line may end there, and whether what is
;; given there is the tab sign's rather than the boundary's.
;;
;; §3.6 makes the space after a sign the sign: a share given at that boundary is a
;; share given to the sign, whose advance §3.6.2 has already decided, so the site is
;; counted -- the room is divided over it like any other -- and what falls to it is
;; not laid down.
(struct boundary (stated after open? absorbed?) #:transparent)

;; The space a jidori forces in, as a list of `(item-index . amount)`.
;;
;; `bases` is the construct's own items in line order and `boundaries` is what
;; stands between them. The cells are full-em cells of the paragraph's own size, and
;; what the run does not fill is shared over its internal boundaries -- the space is
;; between the characters and not around them, because §3.7.3 spreads the run across
;; the cells rather than placing it in one.
(define (jidori-separations cells em bases boundaries style)
  (define want (chk* cells em))
  ;; What the run already occupies is its characters AND the space Table 1 states
  ;; between them: the cells are a width and the run is set into them as it stands,
  ;; so a boundary that already carries a half em has half an em less to be given.
  (define have
    (chk+ (for/fold ([sum 0]) ([one (in-list bases)]) (chk+ sum (cdr one)))
          (for/fold ([sum 0]) ([one (in-list boundaries)]) (chk+ sum (boundary-stated one)))))
  (define room (max 0 (chk- want have)))
  ;; §3.7.3: "The following, however, should be set solid: positions where line
  ;; breaks are prohibited ... These sequences should be treated as a single block."
  ;; A boundary a line may not end at takes none of the room, and a run with no such
  ;; boundary left is one block, which the section's own last sentence then aligns to
  ;; the head of the cells.
  (define open (for/list ([one (in-list boundaries)] #:when (boundary-open? one)) one))
  (define sites (length open))
  (cond
    [(zero? room) '()]
    ;; A run of one character has no internal boundary to share the cells out over,
    ;; and a run whose every boundary is solid has none left. The cells are still what
    ;; the run occupies: §3.7.3 gives the run a width, the run is as wide as it is,
    ;; and the rest of the width stands after it.
    [(<= sites 0) (list (cons (add1 (car (last bases))) room))]
    [else
     (define share (div-trunc room sites))
     (define left (rem-trunc room sites))
     (define trailing? (answer-is? style "adjustment.remainder" "trailing"))
     (for/list ([one (in-list open)] [rank (in-naturals)]
                #:when (not (boundary-absorbed? one)))
       (define extra
         (if trailing?
             (if (>= rank (- sites left)) 1 0)
             (if (< rank left) 1 0)))
       (cons (boundary-after one) (chk+ share extra)))]))

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
;;
;; `offered` is the positions §3.4.2's "a position where line breaking is permitted"
;; leaves open -- the caller's own break opportunities inside the note, where it
;; stated any (docs/decisions/stacked-structure-geometry.md) -- or `#f` where every
;; boundary is a candidate. A note the caller offered nothing inside divides
;; wherever it balances best; one it offered a single position inside divides there
;; whether or not that is the balanced place, because the balance sentence chooses
;; among the permitted positions rather than instead of them.
(define (balanced-split widths [offered #f])
  (define total (for/fold ([sum 0]) ([one (in-list widths)]) (chk+ sum one)))
  (define count (length widths))
  (define (candidate? index) (or (not offered) (null? offered) (and (memv index offered) #t)))
  (cond
    [(<= count 1) count]
    [else
     (let walk ([index 1] [ahead (car widths)] [best #f] [score #f])
       (cond
         [(>= index count) (or best 1)]
         [(not (candidate? index))
          (walk (add1 index) (chk+ ahead (list-ref widths index)) best score)]
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
