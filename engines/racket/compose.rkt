#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; §3.8: a paragraph, as lines.
;;
;; Three things happen here and they are separable.
;;
;; **Where the lines end.** §3.8.1: "within a paragraph, lines are created by
;; separating character sequences at places where line breaking is not prohibited".
;; The caller states which boundaries are opportunities at all; Table 2 and §C.3's
;; four levels say which of those a line may actually be broken at; and the choice
;; among the survivors is a whole-paragraph minimization, because §3.1.12's own
;; worked example compares two candidate *paragraphs* rather than accepting the
;; first line that fits.
;;
;; **How wide each line comes out.** Table 1 states an amount at every boundary and
;; at the two line edges; §3.8.3's ladder takes some of it back where the line is
;; too long and §3.8.4's puts more in where it is too short. Both ladders work in
;; priority stages, and within a stage "equally, with respect to the corresponding
;; character size".
;;
;; **What the answer says.** A placement's `advance` is the advance the line was
;; *composed from* — the character's own, plus the space Table 1 states after it —
;; and its `inline` is where it actually stands once the ladders have run. Those are
;; two different numbers whenever a line was adjusted, and the protocol carries
;; both: `inline_extent` and the inline positions are the adjusted geometry, and the
;; advance is the geometry the adjustment was computed from.
;;
;; The cost function
;;
;; JLReq states no paragraph-level objective (`docs/decisions/adjustment-preference.md`),
;; so the numbers below are this engine's own and are stated rather than derived:
;; the square of a line's shortfall, an overfull line at a thousand times that plus a
;; constant that dominates any number of short lines, a last line at a hundredth
;; because §3.8.1 says its end "need not be aligned", a cap so that one impossible
;; line does not swamp the ordering between two arrangements, a flat charge for
;; taking a discretionary break, and a charge for a last line short of the caller's
;; widow minimum that dominates everything else. Ties go to the arrangement whose
;; first line ends earliest.

(require racket/list
         racket/vector
         (prefix-in tables: "tables.rkt")
         "arith.rkt"
         "model.rkt"
         "style.rkt"
         "classes.rkt"
         "spacing.rkt"
         "kinsoku.rkt"
         "adjust.rkt"
         "ruby.rkt"
         "ornament.rkt"
         "structure.rkt"
         "tabs.rkt")

(provide compose
         (struct-out placed)
         (struct-out attached)
         (struct-out line))

;; One cluster, placed.
(struct placed (index start end inline block advance size frame writing-mode transform) #:transparent)

;; One annotation, placed. `symbol` is the repeated mark an emphasis-dots construct
;; sets and #f for a reading, which carries its own text instead.
(struct attached (construct start end inline block advance size writing-mode transform symbol)
  #:transparent)

;; One line of the answer.
(struct line (start end inline-origin block-origin inline-extent block-extent clusters attachments)
  #:transparent)

;; ----------------------------------------------------------------------------
;; The cost constants
;; ----------------------------------------------------------------------------

;; A line that cannot be composed at all.
(define impossible infinite-cost)

;; What an overfull line costs before its overrun is even counted. It is small on
;; purpose: between two arrangements that both overrun, the one that overruns LESS
;; has to win however many lines it takes to do it, so nothing may be charged per
;; line that a square of the overrun cannot outweigh.
;;
;; It is also bounded from above, and the bound is what stops the ordering from
;; inverting: splitting one line that overruns by `a + b` into two that overrun by
;; `a` and by `b` costs one more charge and saves `2 * weight * a * b` of overrun, so
;; a charge larger than that would prefer the single unsplittable line. The `vertical`
;; and `tate-chu-yoko` censuses reach that comparison at `a = b = 500`, which puts the
;; ceiling at five hundred million; raising `badness-cap` far enough to need a bigger
;; charge than that trades one of those censuses for whatever the raise was for.
(define overfull-charge 1000)

;; ... and its overrun counts a thousand times over, so that between two overfull
;; arrangements the one that overruns less wins.
(define overfull-weight 1000)

;; The most a single line's shortfall may contribute.
(define badness-cap 1000000000)

;; §3.8.1: the last line's end "need not be aligned to the other alignment
;; position", so its shortfall is charged at a hundredth.
(define last-line-divisor 100)

;; Taking a break the caller marked discretionary is a visible act -- a hyphen
;; appears -- so it is charged even where the line it produces is perfect.
(define discretionary-charge 100000)

;; §3.5.4: a last line shorter than the caller's minimum is what widow adjustment
;; exists to avoid, so it outranks every other term.
(define widow-charge 1000000000000000)

;; §3.7.4: on an independent formula line a break before a math operator is the
;; second priority, so it is charged more than any line-length argument can save.
(define operator-break-charge 100000000000)

;; §3.4.3: a warichu cut between two main lines is cut as near its own middle as
;; the caller's own break opportunities allow, and this is what makes the balance
;; of the two halves outrank the balance of the two lines.
(define warichu-balance-weight 10000000000)

;; ----------------------------------------------------------------------------
;; Items
;; ----------------------------------------------------------------------------

;; The transform §3.2 gives one occurrence.
;;
;; Horizontal composition sets everything upright. Vertical composition turns a
;; proportional cluster a quarter turn clockwise (§3.2.6) and leaves a full-em or
;; fixed-width one standing (§3.2.4, where it is Japanese for spacing too).
(define (transform-of para one)
  (cond
    [(not (eq? (paragraph-writing-mode para) 'vertical-rl)) 'identity]
    [(eq? (cluster-frame-of para one) 'proportional) 'rotate-clockwise]
    [else 'identity]))

(define (piece-of para one index transform writing-mode [inline 0] [block 0])
  (piece index
         (cluster-start one)
         (cluster-end one)
         (cluster-advance one)
         (cluster-size-of para one)
         (cluster-frame-of para one)
         transform
         inline
         block
         writing-mode))

(define (plain-item para style one index formulae)
  (define transform (transform-of para one))
  ;; §3.7.4: inside a formula the caller declared, a key Appendix A lists as a math
  ;; symbol or a math operator is one. Outside a formula the same key is read like
  ;; any other character, which is what keeps `=` on the proportional frame Western.
  (define class
    (or (and (for/or ([each (in-list formulae)])
               (and (>= (cluster-start one) (construct-start each))
                    (< (cluster-start one) (construct-end each))))
             (math-class-of (folded-key (cluster-text para one))))
        (classify-cluster para one style)))
  ;; §3.7.4: "The width of math symbols (cl-17) and math operators (cl-18) is
  ;; full-width, i.e. one em", whatever advance the caller shaped them to.
  (define advance
    (if (math-class? class) (extent-inline (cluster-size-of para one)) (cluster-advance one)))
  (item index
        (cluster-start one)
        (cluster-end one)
        advance
        (cluster-size-of para one)
        (cluster-frame-of para one)
        (cluster-role one)
        class
        transform
        ;; §3.6: a tab sign is not a character standing on the line, it is the space
        ;; between two things that are. Nothing beside it is a character adjacency,
        ;; so Table 1 states nothing there.
        (if (tab-character? para one) 'tab 'cluster)
        (list (piece-of para one index transform (paragraph-writing-mode para)))
        #f
        #f
        0
        0
        '()
        (cond
          [(for/or ([each (in-list formulae)])
             (and (>= (cluster-start one) (construct-start each))
                  (< (cluster-start one) (construct-end each))
                  each))
           => (lambda (found) (list 'formula (display-formula? found para)))]
          ;; §3.4: the TEXT of an inline cutting note is what is set on two rows.
          ;; The brackets that close it are characters of the main line -- cl-28 and
          ;; cl-29 are their own classes precisely because they stand where the line
          ;; does -- so a cluster the caller gave the warichu-bracket role is not
          ;; part of the block.
          [(for/or ([each (in-list (paragraph-constructs para))] [at (in-naturals)])
             (and (eq? (construct-kind each) 'warichu)
                  (>= (cluster-start one) (construct-start each))
                  (< (cluster-start one) (construct-end each))
                  ;; `text?` is whether this cluster is set on the note's own rows;
                  ;; a bracket stands on the main line instead. `last?` is whether it
                  ;; is the structure's own last character, which carries no space of
                  ;; its own: what Table 1 states after it stands after the whole
                  ;; block and is not the character's (§3.4).
                  (list 'warichu at
                        (not (eq? (cluster-role one) 'warichu-bracket))
                        (>= (cluster-end one) (construct-end each)))))
           => values]
          [else #f])))

(define (tab-character? para one)
  (define text (source-slice (paragraph-source para) (cluster-start one) (cluster-end one)))
  (and (= (string-length text) 1) (char=? (string-ref text 0) #\tab)))

;; §3.2.5: the clusters of one tate-chu-yoko run are one thing on the line.
;;
;; The run stands across the line rather than along it, so its two measurements are
;; two different numbers: what it takes UP the line is one em of the members' own
;; block size -- the height of the horizontal string, which is what the vertical
;; line has to make room for -- and what it takes ACROSS the line is the sum of the
;; members' advances, which is where the string actually goes. A run of square
;; members makes the two look like one number, and they are not.
;;
;; The class is cl-30 for the whole run and for spacing, breaking and adjustment
;; alike (§C.2 note 13, §E.2 note 12).
(define (run-item para clusters indices index)
  (define widths (for/list ([one (in-list clusters)]) (cluster-advance one)))
  (define across (for/fold ([sum 0]) ([one (in-list widths)]) (chk+ sum one)))
  ;; §3.2.5's own word is "centered", and the half of an odd width the center does
  ;; not divide is taken from the leading side, which is the rounding `div-trunc`
  ;; already does toward zero.
  (define offsets
    (let walk ([rest widths] [at (- (div-trunc across 2))] [out '()])
      (if (null? rest) (reverse out) (walk (cdr rest) (chk+ at (car rest)) (cons at out)))))
  (define members
    (for/list ([one (in-list clusters)] [at (in-list indices)] [block (in-list offsets)])
      (piece-of para one at 'tate-chu-yoko 'horizontal-tb 0 block)))
  ;; What the run takes ALONG the line is the tallest member's block size: the
  ;; string is set across the line, so the line has to be as long as the string is
  ;; tall. What it takes ACROSS the line is the string's own width, or the
  ;; paragraph's own size where the string is narrower than one -- a run of two
  ;; digits is one character's worth of line whatever the digits measure.
  (define along
    (for/fold ([most 0]) ([one (in-list members)])
      (max most (extent-block (piece-size one)))))
  ;; The em the run is spaced against is its members' own, not the height it came
  ;; out at: §3.2.5's half em beside a run is a half em of the size the run is set
  ;; in.
  (define em
    (for/fold ([most 0]) ([one (in-list members)])
      (max most (extent-inline (piece-size one)))))
  (item index
        (cluster-start (car clusters))
        (cluster-end (last clusters))
        along
        (extent em (max across (extent-block (paragraph-size para))))
        (cluster-frame-of para (car clusters))
        #f
        30
        'tate-chu-yoko
        'tate-chu-yoko
        members
        #f #f 0 0 '() #f))

;; The things that stand on the line: the caller's clusters, with each construct's
;; own clusters gathered into the one item that construct is.
(define (items-of para style)
  (define clusters (paragraph-clusters para))
  (define count (vector-length clusters))
  (define formulae (formula-constructs para))
  (define runs (grouping-constructs para))
  (define out '())
  (let walk ([index 0] [next 0])
    (cond
      [(>= index count) (void)]
      [else
       (define found (assv (cluster-start (vector-ref clusters index)) runs))
       (cond
         [found
          (define stop (construct-end (cdr found)))
          (define inside
            (for/list ([at (in-range index count)]
                       #:break (>= (cluster-start (vector-ref clusters at)) stop))
              at))
          (define held (for/list ([at (in-list inside)]) (vector-ref clusters at)))
          (set! out (cons (if (eq? (construct-kind (cdr found)) 'furawake)
                              (furawake-item para style (cdr found) held inside next)
                              (run-item para held inside next))
                          out))
          (walk (+ index (length inside)) (add1 next))]
         [else
          (set! out (cons (plain-item para style (vector-ref clusters index) index formulae) out))
          (walk (add1 index) (add1 next))])]))
  (list->vector (reverse out)))

;; The constructs that gather clusters into one item, as `(start . end)` pairs.
(define (grouping-constructs para)
  (for/list ([one (in-list (paragraph-constructs para))]
             #:when (memq (construct-kind one) '(tate-chu-yoko furawake)))
    (cons (construct-start one) one)))

;; §3.7.2: a furawake is one thing on the line, whose text is set on as many rows
;; as the caller declared columns.
;;
;; The caller's own break opportunities are where it divides. They are not line
;; break opportunities inside it: the whole structure is one position on the main
;; line however many rows it holds, so a break the caller stated inside one is the
;; column division and not a place the main line may end.
(define (furawake-item para style one clusters indices index)
  (define columns (cdr (assq 'columns (construct-payload one))))
  (define gap (cdr (assq 'line-gap (construct-payload one))))
  (define stated
    (for/list ([each (in-list (paragraph-breaks para))]
               #:when (for/or ([piece (in-list clusters)] [rank (in-naturals)])
                        (and (> rank 0) (= (brk-offset each) (cluster-start piece)))))
      (for/first ([piece (in-list clusters)] [rank (in-naturals)]
                  #:when (= (brk-offset each) (cluster-start piece)))
        rank)))
  (define rows (rows-of columns (length clusters) (sort (remove-duplicates stated) <)))
  (define widths
    (for/list ([row (in-list rows)])
      (for/fold ([sum 0]) ([at (in-list row)])
        (chk+ sum (cluster-advance (list-ref clusters at))))))
  (define heights
    (for/list ([row (in-list rows)])
      (for/fold ([most 0]) ([at (in-list row)])
        (max most (extent-block (cluster-size-of para (list-ref clusters at)))))))
  (define vertical? (eq? (paragraph-writing-mode para) 'vertical-rl))
  (define blocks (row-block-offsets heights gap (extent-block (paragraph-size para)) vertical?))
  (define along (for/fold ([most 0]) ([one (in-list widths)]) (max most one)))
  (define across
    (chk+ (for/fold ([sum 0]) ([one (in-list heights)]) (chk+ sum one))
          (chk* gap (max 0 (sub1 (length heights))))))
  (define members
    (append*
     (for/list ([row (in-list rows)] [block (in-list blocks)])
       (let walk ([rest row] [at 0] [out '()])
         (cond
           [(null? rest) (reverse out)]
           [else
            (define piece (list-ref clusters (car rest)))
            (walk (cdr rest)
                  (chk+ at (cluster-advance piece))
                  (cons (piece-of para piece (list-ref indices (car rest))
                                  (transform-of para piece)
                                  (paragraph-writing-mode para)
                                  at block)
                        out))])))))
  (item index
        (cluster-start (car clusters))
        (cluster-end (last clusters))
        along
        ;; The size is the character size the structure is set at, not the block's
        ;; own two extents: it is what every fraction Table 1 states beside the
        ;; structure is a fraction OF, and a block three characters wide is not a
        ;; character three ems wide.
        (cluster-size-of para (car clusters))
        (cluster-frame-of para (car clusters))
        #f
        (classify-cluster para (car clusters) style)
        'identity
        'furawake
        members
        #f #f 0 0 '() #f))

;; ----------------------------------------------------------------------------
;; Break opportunities
;; ----------------------------------------------------------------------------

;; The kind of break the caller states at the boundary before item `index`, or #f.
;;
;; A break is stated by byte offset, and the only offsets that name a boundary
;; between two items are the starts of the items after the first. A break stated
;; anywhere else -- at the paragraph head, at its end, in the middle of a cluster --
;; names no boundary of this paragraph and is carried by nothing.
(define (break-kinds para items)
  (define starts (make-hash))
  (for ([one (in-vector items)] [index (in-naturals)])
    (when (> index 0)
      (hash-set! starts (item-start one) index)))
  (define kinds (make-hash))
  (for ([one (in-list (paragraph-breaks para))])
    (define index (hash-ref starts (brk-offset one) #f))
    (when index
      ;; A mandatory break outranks an ordinary one stated at the same boundary.
      (define standing (hash-ref kinds index #f))
      (when (or (not standing) (eq? (brk-kind one) 'mandatory))
        (hash-set! kinds index (brk-kind one)))))
  kinds)

;; ----------------------------------------------------------------------------
;; §3.4: the inline cutting note, on the line it lands on
;; ----------------------------------------------------------------------------

;; Where one warichu's own characters stand inside the block they make.
(struct spot (run last inline block advance) #:transparent)

;; The runs of warichu text on one line, as `(first . last)` pairs of item indices.
;;
;; A note is cut between two MAIN lines at one of the caller's own break
;; opportunities (§3.4.3), so what is on this line is a stretch of the note and not
;; necessarily all of it -- and the stretch divides itself into two rows again.
(define (warichu-runs items first last)
  (let walk ([index first] [out '()])
    (cond
      [(> index last) (reverse out)]
      [(not (warichu-of (vector-ref items index))) (walk (add1 index) out)]
      [else
       (define name (warichu-of (vector-ref items index)))
       (define stop
         (let ahead ([at index])
           (if (and (<= (add1 at) last) (equal? (warichu-of (vector-ref items (add1 at))) name))
               (ahead (add1 at))
               at)))
       (walk (add1 stop) (cons (cons index stop) out))])))

;; The note whose ROWS this item is set on, or #f. A bracket belongs to the
;; construct and stands on the main line, so it is not one of these.
(define (warichu-of one)
  (let ([found (item-structure one)])
    (and found
         (eq? (car found) 'warichu)
         (= (length found) 4)
         (caddr found)
         (list 'warichu (cadr found)))))

;; The layout of every warichu run on one line.
;;
;; Answers a hash from item index to its own place inside the block, and two more
;; from the run's first item to how much of the line the block takes and how much
;; of the block axis it needs.
(define (warichu-layout para style items first last)
  (define vertical? (eq? (paragraph-writing-mode para) 'vertical-rl))
  (define spots (make-hasheqv))
  (define widths (make-hasheqv))
  (define heights (make-hasheqv))
  (for ([run (in-list (warichu-runs items first last))])
    (define indices (for/list ([index (in-range (car run) (add1 (cdr run)))]) index))
    (define split (balanced-split (for/list ([index (in-list indices)])
                                    (item-advance (vector-ref items index)))))
    (define rows
      (if (<= (length indices) 1)
          (list indices)
          (list (take indices split) (drop indices split))))
    ;; §B.2 note 13: a Western word space at the edge of a SUBLINE occupies no
    ;; space, exactly as one at the edge of a main line does.
    (define (own index rank size)
      (if (and (word-space? (vector-ref items index)) (or (= rank 0) (= rank (sub1 size))))
          0
          (item-advance (vector-ref items index))))
    (define row-widths
      (for/list ([row (in-list rows)])
        (define size (length row))
        (for/fold ([sum 0]) ([index (in-list row)] [rank (in-naturals)])
          (chk+ (chk+ sum (own index rank size))
                (if (< (add1 rank) size)
                    (if (or (zero? (own index rank size))
                            (zero? (own (list-ref row (add1 rank)) (add1 rank) size)))
                        0
                        (total-of (boundary-contributions (vector-ref items index)
                                                          (vector-ref items (list-ref row (add1 rank)))
                                                          (paragraph-writing-mode para)
                                                          style)))
                    0)))))
    (define row-heights
      (for/list ([row (in-list rows)])
        (for/fold ([most 0]) ([index (in-list row)])
          (max most (extent-block (item-size (vector-ref items index)))))))
    (define blocks
      (row-block-offsets row-heights 0 (extent-block (paragraph-size para)) vertical?))
    (for ([row (in-list rows)] [block (in-list blocks)])
      (define size (length row))
      (let walk ([rest row] [rank 0] [at 0])
        (unless (null? rest)
          (define index (car rest))
          (define advance (own index rank size))
          (hash-set! spots index (spot (car run) (cdr run) at block advance))
          (walk (cdr rest)
                (add1 rank)
                (chk+ (chk+ at advance)
                      (if (< (add1 rank) size)
                          (if (or (zero? advance)
                                  (zero? (own (list-ref row (add1 rank)) (add1 rank) size)))
                              0
                              (total-of (boundary-contributions
                                         (vector-ref items index)
                                         (vector-ref items (list-ref row (add1 rank)))
                                         (paragraph-writing-mode para)
                                         style)))
                          0))))))
    (hash-set! widths (car run) (for/fold ([most 0]) ([one (in-list row-widths)]) (max most one)))
    (hash-set! heights (car run)
               (for/fold ([sum 0]) ([one (in-list row-heights)]) (chk+ sum one))))
  (values spots widths heights))

;; ----------------------------------------------------------------------------
;; One line, measured
;; ----------------------------------------------------------------------------

;; A line as the ladders see it: the advance of every item, the amount at every
;; boundary and at the two edges, and what may be given back or taken up where.
;; `fixed` is the space §3.3 forced in at each of the same positions. It is not a
;; Table 1 amount and neither ladder touches it: a reading that had nowhere to go is
;; not an adjustment site, it is the reason the line is the width it is.
;; `slack` is how far short of the measure the line came BEFORE §3.8.4 opened it
;; out, which is the number a paragraph is judged on: justification hides the
;; difference between a line that was nearly full and one that was half empty, and
;; §3.8.1 is about where the lines end rather than about how they were stretched.
;; `cut` is §3.6.3's fourth case: a tab sign on this line has run its stops out, so
;; the line has to end before it and this arrangement is not one.
(struct shape (advances gaps fixed extent overrun hung slack cut) #:transparent)

;; §3.2.2 and §B.2 note 13: a Western word space at the line head or the line end
;; occupies no visible space, and gets its width back the moment the same text sits
;; elsewhere on a line.
;;
;; The rule reaches the space §3.2.2 is about and no other. That section's own
;; subject is "mixed text composition ... the basic approach is to use proportional
;; Western fonts", and the one third em it gives cl-26 is a proportional width; an
;; occurrence of U+0020 the caller set on the full em or the fixed width is not the
;; space that section measured, and it keeps the advance the caller shaped it to.
(define (word-space? one)
  (and (= (item-class one) 26) (eq? (item-frame one) 'proportional)))

(define (collapses-at-edge? one edge?)
  (and edge? (word-space? one)))

;; `gaps` is one longer than `advances` by one on each side: index 0 is the line
;; head, index i the boundary before item i of the line, and the last the line end.
(define (measure-line para style items first last first-line? last-line? alignment)
  (define count (add1 (- last first)))
  (define writing-mode (paragraph-writing-mode para))
  (define indent (if first-line? (paragraph-first-line-indent para) 0))
  ;; Two geometries, and the difference between them is §B.2 note 13 alone. `raw`
  ;; is what the caller shaped and what Table 1 states; the other is that with a
  ;; word space the line edge collapsed -- its own advance and the boundary beside
  ;; it both gone. The line is composed from the second and the ladders measure
  ;; their room in the first, which is what lets a collapsed space still give back
  ;; the width it would have had.
  (define-values (spots stack-widths stack-heights) (warichu-layout para style items first last))
  ;; §3.4: a run of warichu text is ONE position on the line however many characters
  ;; it holds. Its first item carries the block's whole width along the line and the
  ;; rest carry nothing, which is also what keeps the boundaries inside it out of
  ;; both ladders.
  ;; The block's own width goes on the LAST item of the run, so that every item of
  ;; it starts from the same cursor -- which is what a block is: one position on the
  ;; line that several characters stand inside.
  (define (stacked offset)
    (define index (+ first offset))
    (define found (hash-ref spots index #f))
    (and found (if (= index (spot-last found)) (hash-ref stack-widths (spot-run found)) 0)))
  (define shaped
    (for/vector ([offset (in-range count)])
      (define one (vector-ref items (+ first offset)))
      (or (stacked offset)
          (if (collapses-at-edge? one (or (= offset 0) (= offset (sub1 count)))) 0 (item-advance one)))))
  (define next (and (< (add1 last) (vector-length items)) (vector-ref items (add1 last))))
  (define gap-terms
    (for/vector ([index (in-range (add1 count))])
      (cond
        [(= index 0) (head-contributions (vector-ref items first) style first-line? writing-mode)]
        [(= index count) (end-contributions (vector-ref items last) style writing-mode next)]
        [(after-head-space? items first count index) '()]
        ;; Inside one warichu block the boundaries are the block's own business:
        ;; the note is one position on the main line and the space between two of
        ;; its characters is inside that position rather than beside it. The
        ;; boundary where the block BEGINS is on the line, and Table 1 states it.
        [(let ([before (hash-ref spots (+ first index -1) #f)]
               [after (hash-ref spots (+ first index) #f)])
           (and before after (= (spot-run before) (spot-run after))))
         '()]
        [else
         (boundary-contributions (vector-ref items (+ first index -1))
                                 (vector-ref items (+ first index))
                                 writing-mode
                                 style)])))
  ;; The same boundaries as the transcription states them, with no section's own
  ;; withdrawal applied. §3.2.5 takes six of Table 1's cl-30 cells out of the
  ;; *spacing* and Appendices D and E go on reading them at face value, so a run on
  ;; a line that had to give space back ends up a quarter em inside the character
  ;; before it; §B.2 note 13 does the same to the boundary beside a collapsed word
  ;; space. What the ladders measure their room in is this.
  (define raw-gap-terms
    (for/vector ([index (in-range (add1 count))])
      (cond
        [(or (= index 0) (= index count)) (vector-ref gap-terms index)]
        [else
         (boundary-contributions (vector-ref items (+ first index -1))
                                 (vector-ref items (+ first index))
                                 writing-mode
                                 style
                                 #t)])))
  (define gaps (vector-map total-of gap-terms))
  (define fixed
    (for/vector ([index (in-range (add1 count))])
      (cond
        [(< index count) (item-separation (vector-ref items (+ first index)))]
        [else (item-tail (vector-ref items last))])))
  (define forced (for/fold ([sum 0]) ([one (in-vector fixed)]) (chk+ sum one)))
  ;; §3.6: a tab sign takes up whatever space puts what follows it at a stop, which
  ;; is not known until the line is walked -- so the walk happens here, once the
  ;; advances and the amounts beside them are known. It changes the advance of the
  ;; signs and of nothing else.
  (define-values (advances cut?)
    (tab-walk para items first last count shaped gaps fixed indent))
  (define natural
    (+ indent
       forced
       (for/fold ([sum 0]) ([one (in-vector advances)]) (chk+ sum one))
       (for/fold ([sum 0]) ([one (in-vector gaps)]) (chk+ sum one))))
  (define measure (paragraph-line-extent para))
  (define room (- measure natural))
  (cond
    [(negative? room)
     (define-values (kept-advances kept-gaps left)
       (reduce para style items first last advances gaps advances raw-gap-terms (- room)))
     (define reduced (+ indent forced (sum-of kept-advances) (sum-of kept-gaps)))
     (define hung (hang-of para style items last (- reduced measure)))
     (shape kept-advances kept-gaps fixed (- reduced hung) (max 0 (- reduced hung measure))
            hung 0 cut?)]
    [(and (positive? room) (eq? alignment 'justify) (not last-line?))
     (define-values (open-advances open-gaps)
       (expand para style items first last advances gaps raw-gap-terms room))
     (shape open-advances open-gaps fixed
            (+ indent forced (sum-of open-advances) (sum-of open-gaps)) 0 0 room cut?)]
    [else (shape advances gaps fixed natural 0 0 (max 0 room) cut?)]))

;; §3.6.3: what every tab sign of one line takes up, and whether the line has to end
;; before one of them.
;;
;; The walk is left to right because a sign's own advance depends on where the sign
;; stands, which depends on every advance before it -- an earlier sign's included.
(define (tab-walk para items first last count shaped gaps fixed indent)
  (define signs
    (for/list ([offset (in-range count)]
               #:when (tab-sign? para (vector-ref items (+ first offset))))
      offset))
  (cond
    [(null? signs) (values shaped #f)]
    [else
     (define stops (stops-in-order para))
     (define out (vector-copy shaped))
     (define cut #f)
     (let walk ([offset 0] [cursor (chk+ (chk+ indent (vector-ref gaps 0)) (vector-ref fixed 0))])
       (when (< offset count)
         (when (memv offset signs)
           (define ahead
             (for/first ([one (in-list stops)] #:when (> (tab-stop-position one) cursor)) one))
           (cond
             ;; §3.6.3: a sign standing inside a construct keeps its line and takes
             ;; one em where it stands. Every construct is at least one object on
             ;; the line, so a coordinate inside one is not a position a stop could
             ;; name; a construct that begins or ends exactly at the sign leaves the
             ;; sign beside it rather than in it.
             [(inside-construct? para (vector-ref items (+ first offset)))
              (vector-set! out offset (extent-inline (paragraph-size para)))]
             [ahead
              (define target
                (tab-target ahead (string-after para items first count offset out gaps fixed)))
              ;; The sign takes up the whole distance to where the string goes.
              ;; Nothing stands between the two: Table 1 states no amount after a
              ;; sign, because the amount after a sign is the sign.
              (define beside (vector-ref fixed (add1 offset)))
              (vector-set! out offset (max 0 (chk- (chk- target cursor) beside)))]
             ;; §3.6.3's fourth case has nothing to say to a sign at the line head:
             ;; there is no earlier boundary to send the string back to. It keeps its
             ;; line and takes one em of the paragraph's own size.
             [(zero? offset) (vector-set! out offset (extent-inline (paragraph-size para)))]
             [else (set! cut #t)]))
         (walk (add1 offset)
               (chk+ (chk+ (chk+ cursor (vector-ref out offset)) (vector-ref gaps (add1 offset)))
                     (vector-ref fixed (add1 offset))))))
     (values out cut)]))

;; Whether the item stands strictly inside a construct the caller declared.
(define (inside-construct? para one)
  (for/or ([each (in-list (paragraph-constructs para))])
    (and (> (item-start one) (construct-start each))
         (< (item-end one) (construct-end each)))))

;; The string one sign puts at its stop: what stands between it and the next sign,
;; or the line end, as `(width . text)` pairs.
(define (string-after para items first count offset advances gaps fixed)
  (let walk ([at (add1 offset)] [out '()])
    (cond
      [(>= at count) (reverse out)]
      [(tab-sign? para (vector-ref items (+ first at))) (reverse out)]
      [else
       (define one (vector-ref items (+ first at)))
       ;; What the character takes up on the line, which is its own advance and the
       ;; space that stands after it: §3.6.2's stop is a position the STRING is put
       ;; at, and the string is as wide as the line makes it.
       (walk (add1 at)
             (cons (cons (chk+ (chk+ (vector-ref advances at) (vector-ref gaps (add1 at)))
                               (vector-ref fixed (add1 at)))
                         (source-slice (paragraph-source para) (item-start one) (item-end one)))
                   out))])))

(define (sum-of values)
  (for/fold ([sum 0]) ([one (in-vector values)]) (chk+ sum one)))

;; §3.8.2's hanging punctuation: a full stop or a comma at the line end is set
;; outside the measure rather than the line being adjusted around it.
;;
;; It closes what §3.8.3's ladder could not, and no more: the method is a way of
;; avoiding an adjustment, so it takes exactly the overrun that is left and stops at
;; the character's own advance.
(define (hang-of para style items last over)
  (cond
    [(not (answer-is? style "adjustment.hanging_punctuation" "hanging")) 0]
    [(<= over 0) 0]
    [else
     (define one (vector-ref items last))
     (if (or (= (item-class one) 6) (= (item-class one) 7))
         (min over (item-advance one))
         0)]))

;; ----------------------------------------------------------------------------
;; §3.8.3, the reduction ladder
;; ----------------------------------------------------------------------------

;; One place a line may give space back: how much, at which priority, and against
;; which character size.
;;
;; A boundary is one site where Table 1 states one term and two where it states two,
;; because §D.2 notes 1 through 3 give the two terms of such a boundary two
;; different priorities: a comma before a middle dot has the dot's own quarter em at
;; the fourth stage and the comma's own half em at the fifth, and Table 3 states one
;; cell for the pair. What separates them is the coordinate each term would stand at
;; on its own -- `(cl-07, cl-19)` for the comma's trailing space and
;; `(cl-19, cl-05)` for the dot's leading one -- and reading the stage there is what
;; recovers the two priorities the notes state. The capacities add up to the cell's
;; own, which is the check that the two readings are the same reading.
(struct opening (where index capacity stage two-valued? em) #:transparent)

;; The line edge Tables 3 through 5 address a term's own coordinate by. A term the
;; preceding character owns stands against an ideograph after it; one the trailing
;; character owns stands against an ideograph before it.
(define neutral-class 19)

(define (reduction-sites para style items first last advances gaps raw-advances raw-gap-terms)
  (define count (vector-length advances))
  (define table (reduction-table-of style))
  (append
   ;; §3.8.3 (a) and §D's own first stage: the Western word spaces, all at once.
   ;; §D.2 note 4 takes the line end out: a space with no word after it is not a
   ;; space between words, and there is no visible width there to reduce.
   (for/list ([offset (in-range count)]
              #:when (and (word-space? (vector-ref items (+ first offset)))
                          (< offset (sub1 count))
                          (> (vector-ref raw-advances offset) 0)))
     (define em (item-em (vector-ref items (+ first offset))))
     (opening 'advance offset (max 0 (- (vector-ref raw-advances offset) (word-space-floor em))) 1 #f em))
   ;; Tables 3 through 5, which are "the second and subsequent stages". The line
   ;; head is not among them: all three tables prohibit reduction there.
   (append*
    (for/list ([index (in-range 1 (add1 count))])
      (define terms (vector-ref raw-gap-terms index))
      (define before-class
        (if (= index count) (item-class (vector-ref items last)) (item-class (vector-ref items (+ first index -1)))))
      (define after-class
        (if (= index count) line-edge (item-class (vector-ref items (+ first index)))))
      (gap-openings table items first index terms before-class after-class)))))

;; The openings one boundary offers.
(define (gap-openings table items first index terms before-class after-class)
  (define found (reduction-of table before-class after-class terms))
  (define room (- (total-of terms) (reduction-floor found)))
  (cond
    [(or (not (reduction-stage found)) (<= room 0)) '()]
    [(<= (length terms) 1)
     (list (opening 'gap
                    index
                    room
                    (reduction-stage found)
                    (reduction-two-valued? found)
                    (term-em items first index (car terms))))]
    [else
     ;; Two terms, two priorities. The capacity is taken from the trailing term
     ;; first, which is the order §D.2's own notes take it in.
     (let walk ([rest (reverse terms)] [left room] [out '()])
       (cond
         [(or (null? rest) (<= left 0)) (reverse out)]
         [else
          (define one (car rest))
          (define own
            (if (eq? (contribution-owner one) 'after)
                (reduction-of table neutral-class after-class (list one))
                (reduction-of table before-class neutral-class (list one))))
          (define share (min left (max 0 (- (contribution-amount one) (reduction-floor own)))))
          (walk (cdr rest)
                (- left share)
                (if (and (> share 0) (reduction-stage own))
                    (cons (opening 'gap index share (reduction-stage own) (reduction-two-valued? own)
                                   (term-em items first index one))
                          out)
                    out))]))]))

;; The character size one term is measured against.
(define (term-em items first index one)
  (if (eq? (contribution-owner one) 'after)
      (item-em (vector-ref items (+ first index)))
      (item-em (vector-ref items (+ first index -1)))))

;; Give back `wanted`, in stage order, and report what is left.
(define (reduce para style items first last advances gaps raw-advances raw-gap-terms wanted)
  (define kept-advances (vector-copy advances))
  (define kept-gaps (vector-copy gaps))
  (define sites
    (for/list ([one (in-list (reduction-sites para style items first last advances gaps
                                              raw-advances raw-gap-terms))]
               #:when (and (opening-stage one) (> (opening-capacity one) 0)))
      one))
  (define taken (make-vector (length sites) 0))
  (define trailing? (answer-is? style "adjustment.remainder" "trailing"))
  (define left
    (for/fold ([left wanted]) ([stage (in-range 1 (add1 tables:max-stage))])
      (define here
        (for/list ([one (in-list sites)] [at (in-naturals)] #:when (eqv? (opening-stage one) stage))
          (cons one at)))
      (cond
        [(or (<= left 0) (null? here)) left]
        [else (take-back sites taken here left trailing?)])))
  (for ([one (in-list sites)] [at (in-naturals)])
    (define amount (vector-ref taken at))
    (when (> amount 0)
      (if (eq? (opening-where one) 'advance)
          (vector-set! kept-advances (opening-index one) (- (vector-ref kept-advances (opening-index one)) amount))
          (vector-set! kept-gaps (opening-index one) (- (vector-ref kept-gaps (opening-index one)) amount)))))
  (values kept-advances kept-gaps left))

;; Take `wanted` out of one stage's sites, and report what could not be taken.
;;
;; §3.1.9's two-valued sites are the exception to "equally": a half em after a
;; closing bracket at the line end is a half em or it is nothing, and no size in
;; between, so such a site is closed outright as soon as the line needs anything at
;; all -- which can leave the line shorter than the measure asked for, and does.
(define (take-back sites taken here wanted trailing?)
  (define-values (jumps rest) (partition (lambda (pair) (opening-two-valued? (car pair))) here))
  (define after-jumps
    (for/fold ([left wanted]) ([pair (in-list jumps)])
      (cond
        [(<= left 0) left]
        [else
         (define room (- (opening-capacity (car pair)) (vector-ref taken (cdr pair))))
         (vector-set! taken (cdr pair) (opening-capacity (car pair)))
         (- left room)])))
  (let spread ([left after-jumps] [open rest])
    (cond
      [(<= left 0) 0]
      [(null? open) left]
      [else
       (define shares (share-out left (map (lambda (pair) (opening-em (car pair))) open) trailing?))
       (define got
         (for/fold ([got 0]) ([pair (in-list open)] [want (in-list shares)])
           (define room (- (opening-capacity (car pair)) (vector-ref taken (cdr pair))))
           (define now (min want room))
           (vector-set! taken (cdr pair) (+ (vector-ref taken (cdr pair)) now))
           (+ got now)))
       (define remaining
         (filter (lambda (pair) (> (- (opening-capacity (car pair)) (vector-ref taken (cdr pair))) 0)) open))
       (cond
         [(zero? got) left]
         [else (spread (- left got) remaining)])])))

;; `total` split over sites of the stated character sizes.
;;
;; §3.8.3 and §3.8.4 both say the share is taken or given "equally, with proportional
;; character size", so a boundary between two ems of one size takes twice what a
;; boundary between two of half that size does. The division is exact integer
;; arithmetic and rarely comes out even; `adjustment.remainder` is the question of
;; which end of the line the units that are left over go to, and `leading` -- the
;; answer every profile gives -- puts them at the first sites.
(define (share-out total ems trailing?)
  (define scale (for/fold ([sum 0]) ([em (in-list ems)]) (chk+ sum em)))
  (define base
    (if (positive? scale)
        (for/list ([em (in-list ems)]) (div-trunc (chk* total em) scale))
        (for/list ([em (in-list ems)]) 0)))
  (define spare (- total (for/fold ([sum 0]) ([one (in-list base)]) (chk+ sum one))))
  (define ranks (if trailing? (reverse (build-list (length ems) values)) (build-list (length ems) values)))
  (define bonus (make-vector (length ems) 0))
  (for ([rank (in-list ranks)] [step (in-naturals)] #:when (< step spare))
    (vector-set! bonus rank 1))
  (for/list ([one (in-list base)] [index (in-naturals)])
    (chk+ one (vector-ref bonus index))))

;; ----------------------------------------------------------------------------
;; §3.8.4, the expansion ladder
;; ----------------------------------------------------------------------------

;; One place a line may take space up. `ceiling` is #f where step (d) is the only
;; thing that reaches it, which is unbounded. `em` is the character size §3.8.4's own
;; "equally with proportional character size" measures the share against.
(struct widening (where index ceiling stage em) #:transparent)

(define (expansion-sites para style items first last advances gaps raw-gap-terms)
  (define count (vector-length advances))
  (append
   ;; §B.2 note 13: a space the line edge collapsed has no visible width, and §3.8.4
   ;; (a) opens the space between two words rather than the edge of a line.
   (for/list ([offset (in-range count)]
              #:when (and (word-space? (vector-ref items (+ first offset)))
                          (> offset 0)
                          (< offset (sub1 count))))
     (widening 'advance offset (word-space-ceiling (item-em (vector-ref items (+ first offset)))) 1
               (item-em (vector-ref items (+ first offset)))))
   (append*
    (for/list ([index (in-range 1 count)])
      (define before (vector-ref items (+ first index -1)))
      (define after (vector-ref items (+ first index)))
      (define em (boundary-em (vector-ref raw-gap-terms index) before after))
      (define found (expansion-of (item-class before) (item-class after) em))
      (define ceiling (latin-ceiling style (item-class before) (item-class after) em found))
      (cond
        ;; §B.2 note 13: a word space the line edge collapsed is not on the line,
        ;; and a boundary beside something that is not there is not a place to put
        ;; space.
        [(collapsed-here? items first count index) '()]
        ;; §B.2 notes 9 through 11 and §E.2 notes 5 through 7: the opportunity is
        ;; between two characters of DIFFERENT complexes. Inside one there is none.
        [(same-complex? before after) '()]
        ;; §E.2 note 10: no expansion between a quantity symbol or a European
        ;; numeral and the postfixed abbreviation (cl-13) it holds on to.
        [(and (= (item-class before) 27)
              (= (item-class after) 13)
              (quantity-or-numeral? para before))
         '()]
        ;; §E.2 note 4: the opportunity is between two inseparable characters (cl-08)
        ;; "which are of different kinds", and Table 6 states one cell for the pair.
        [(and (= (item-class before) 8)
              (= (item-class after) 8)
              (equal? (inseparable-kind (single-key para before))
                      (inseparable-kind (single-key para after))))
         '()]
        ;; §3.8.4's own Note, where `adjustment.japanese_latin_expansion_ceiling`
        ;; answers `rigid`: the quarter em between Japanese text and Latin text is a
        ;; fixed size and no step opens it, step (d) included.
        [(and (not ceiling) (latin-coordinate? (item-class before) (item-class after))) '()]
        [else
         (append (if (expansion-stage found)
                     (list (widening 'gap index ceiling (expansion-stage found) em))
                     '())
                 (if (expansion-residual? found)
                     (list (widening 'gap index #f 4 em))
                     '()))])))))

;; The em a boundary's own space is measured in.
;;
;; §B.1 gives every Table 1 term an owner, and Appendices D and E state their floors
;; and ceilings "with respect to the corresponding character size" without naming
;; one -- so the size is the one the amount that is already there was measured
;; against, and the preceding character's where Table 1 states no amount at all.
(define (boundary-em terms before after)
  (cond
    [(null? terms) (item-em before)]
    [(eq? (contribution-owner (car terms)) 'after) (item-em after)]
    [else (item-em before)]))

;; The two coordinates §3.8.4's own Note governs: the quarter em between an
;; ideographic character (cl-19) and a Western one (cl-27).
;;
;; The section names three Japanese classes and three Latin ones, which is nine
;; coordinates in each direction; its Note -- the sentence the `rigid` answer comes
;; from -- names 漢字等（cl-19）など and the three Latin classes. The narrower
;; reading is the one the reference engines answer: at the other sixteen stage-two
;; coordinates Table 6's own half em stands whatever the style says.
(define (latin-coordinate? before after)
  (or (and (= before 19) (= after 27)) (and (= before 27) (= after 19))))

;; The ceiling that coordinate takes, or #f where the style makes it rigid.
(define (latin-ceiling style before after em found)
  (cond
    [(not (latin-coordinate? before after)) (expansion-ceiling found)]
    [(answer-is? style "adjustment.japanese_latin_expansion_ceiling" "rigid") #f]
    [(answer-is? style "adjustment.japanese_latin_expansion_ceiling" "third-em") (third-em em)]
    [else (expansion-ceiling found)]))

;; Whether the item is the bracket that closes an inline cutting note. §3.4 makes
;; that bracket the last character of the STRUCTURE rather than of the line.
(define (closes-structure? one)
  (let ([found (item-structure one)])
    (and found
         (eq? (car found) 'warichu)
         (= (length found) 4)
         (not (caddr found))
         (cadddr found))))

;; Whether two items are two characters of one base character group. §3.3 sets such
;; a group as one thing: no space stands between two of its characters and none
;; opens there, whatever the matrices state at the coordinate.
(define (same-complex? before after)
  (or (and (item-complex before) (equal? (item-complex before) (item-complex after)))
      ;; The boundaries inside an inline cutting note are inside the block and not
      ;; on the line, so neither ladder reaches them: the note is one position on
      ;; the line and the line adjusts around it.
      (and (warichu-of before) (equal? (warichu-of before) (warichu-of after)))))

;; Whether the boundary before offset `index` touches a word space the line edge
;; collapsed. §3.2.2's space is restored the moment the same text sits elsewhere on
;; a line, so this is a fact about the line and not about the cluster.
(define (collapsed-here? items first count index)
  (define (collapsed? offset)
    (and (or (= offset 0) (= offset (sub1 count)))
         (word-space? (vector-ref items (+ first offset)))))
  (or (collapsed? (sub1 index)) (collapsed? index)))

;; Whether the boundary before offset `index` is the one *after* a word space the
;; line head collapsed.
;;
;; §B.2 note 13 takes the space itself out of the line, and at the line head it
;; takes the boundary beside it too: what the first character of the line then has
;; before it is the head of the line and not a space of any width. At the line end
;; the boundary is left alone -- the character before the space still has whatever
;; Table 1 states after it, which is how a closing bracket keeps its half em when a
;; word space follows it onto the line end.
(define (after-head-space? items first count index)
  (and (> count 1)
       (= index 1)
       (word-space? (vector-ref items first))))

;; Take up `room`, in stage order.
(define (expand para style items first last advances gaps raw-gap-terms room)
  (define open-advances (vector-copy advances))
  (define open-gaps (vector-copy gaps))
  (define sites (expansion-sites para style items first last advances gaps raw-gap-terms))
  (define trailing? (answer-is? style "adjustment.remainder" "trailing"))
  (for/fold ([left room] #:result (values open-advances open-gaps))
            ([stage (in-range 1 5)])
    (define here (filter (lambda (one) (= (widening-stage one) stage)) sites))
    (cond
      [(or (<= left 0) (null? here)) left]
      [else (give-out open-advances open-gaps here left trailing?)])))

(define (open-current advances gaps one)
  (if (eq? (widening-where one) 'advance)
      (vector-ref advances (widening-index one))
      (vector-ref gaps (widening-index one))))

(define (open-set! advances gaps one value)
  (if (eq? (widening-where one) 'advance)
      (vector-set! advances (widening-index one) value)
      (vector-set! gaps (widening-index one) value)))

;; Put `room` into one stage's sites, and report what would not go in.
(define (give-out advances gaps sites room trailing?)
  (let spread ([left room] [open sites])
    (cond
      [(<= left 0) 0]
      [(null? open) left]
      [else
       (define shares (share-out left (map widening-em open) trailing?))
       (define given
         (for/fold ([given 0]) ([one (in-list open)] [want (in-list shares)])
           (define ceiling (widening-ceiling one))
           (define now
             (if ceiling (max 0 (min want (- ceiling (open-current advances gaps one)))) want))
           (open-set! advances gaps one (+ (open-current advances gaps one) now))
           (+ given now)))
       (define remaining
         (filter (lambda (one)
                   (define ceiling (widening-ceiling one))
                   (or (not ceiling) (> (- ceiling (open-current advances gaps one)) 0)))
                 open))
       (cond
         [(zero? given) left]
         [else (spread (- left given) remaining)])])))

;; ----------------------------------------------------------------------------
;; The paragraph
;; ----------------------------------------------------------------------------

(define (compose para)
  (define style (resolve-style (paragraph-style para)))
  (define items (with-ruby para style (items-of para style)))
  (define count (vector-length items))
  (cond
    [(zero? count) (values '() '())]
    [else
     (define kinds (break-kinds para items))
     (define alignment (or (paragraph-alignment para) 'justify))
     (define permitted (permitted-breaks para style items kinds))
     ;; §3.5.4: the widow minimum is a preference and not a constraint. The
     ;; paragraph is arranged under it first, and where no arrangement satisfies it
     ;; -- a minimum the caller set past the text, or a paragraph with nowhere to
     ;; take the clusters from -- the arrangement it would have had stands, and the
     ;; answer says so.
     ;; §3.5.4: a last line shorter than the caller's minimum is bought off by
     ;; SHORTENING THE LINE BEFORE IT, which is a paragraph of the same lines
     ;; arranged differently. So the paragraph is arranged first without the
     ;; minimum, and the minimum is then allowed to rearrange it -- but only into a
     ;; paragraph of the same number of lines, because taking a line away is not
     ;; shortening the one before the last. Where it cannot, the arrangement stands
     ;; and the answer says the last line is short.
     (define wanted (paragraph-widow-minimum para))
     (define natural (choose-breaks para style items permitted kinds alignment #f))
     (define bought
       (and wanted
            (not (widow-satisfied? natural wanted))
            (let ([found (choose-breaks para style items permitted kinds alignment wanted)])
              (and (widow-satisfied? found wanted)
                   (= (length found) (length natural))
                   found))))
     (define kept (or bought natural))
     (lay-out para style items kept alignment
              (and wanted (not (widow-satisfied? kept wanted))))]))

;; Whether the last line of an arrangement holds the minimum the caller asked for.
(define (widow-satisfied? breaks wanted)
  (and (pair? breaks)
       (let ([final (last breaks)])
         (>= (add1 (- (cdr final) (car final))) wanted))))

;; ----------------------------------------------------------------------------
;; Ruby
;; ----------------------------------------------------------------------------

;; The items again, with what §3.3 does to them: the class of a base character, the
;; complex and the run it belongs to, the space a reading forced in before it, and
;; the reading itself.
;; §3.4.2 and §3.9.2: the brackets that close an inline cutting note are cl-28 and
;; cl-29, which are their own classes precisely because they stand where the line
;; does and are set against it differently from ordinary brackets. The caller names
;; them with the `warichu-bracket` role, and the role is what settles it: §A.28 and
;; §A.29 enumerate the parentheses, and a note bracketed with anything else is still
;; a note bracketed.
(define (warichu-bracket-class one items index)
  (define found (item-structure one))
  (and found
       (eq? (car found) 'warichu)
       (= (length found) 4)
       (not (caddr found))
       (if (cadddr found) 29 28)))

(define (with-ruby para style items)
  (define constructs (ruby-constructs para))
  (define ornaments (ornament-constructs para))
  (cond
    [(null? (paragraph-constructs para)) items]
    [else
     (define count (vector-length items))
     ;; Which construct, and which of its runs, each item's own cluster belongs to.
     (define marks (make-hasheqv))
     (for ([one (in-list constructs)])
       (for ([piece (in-list (ruby-runs one))] [rank (in-naturals)])
         (for ([index (in-range count)])
           (define found (vector-ref items index))
           (when (and (>= (item-start found) (run-base-start piece))
                      (< (item-start found) (run-base-end piece)))
             (hash-set! marks index (list one rank))))))
     (define reclassified
       (for/vector ([one (in-vector items)] [index (in-naturals)])
         (define found (hash-ref marks index #f))
         (cond
           [(not found) one]
           [else
            (define construct (first found))
            (struct-copy item one
                         [class (ruby-class-of construct)]
                         ;; §B.2 notes 10 and 11 and §E.2 notes 6 and 7 are stated
                         ;; about one complex. A simple-ruby complex is one run; a
                         ;; jukugo compound is the whole construct, which is what
                         ;; keeps its own internal boundary from opening while a
                         ;; mono run's does.
                         [complex (if (eq? (ruby-kind construct) 'jukugo)
                                      (list 'ruby (ruby-index construct))
                                      (list 'ruby (ruby-index construct) (second found)))]
                         [run (list 'ruby (ruby-index construct) (second found))])])))
     (define ornamented
       (for/vector ([one (in-vector reclassified)] [index (in-naturals)])
         (define found
           (for/or ([each (in-list ornaments)])
             (and (>= (item-start one) (ornament-start each))
                  (< (item-start one) (ornament-end each))
                  each)))
         (cond
           [(not found) one]
           [else
            ;; §3.3.9 makes each base character its own ornamented character
            ;; complex and §3.7.1 makes the whole construct one. That is the only
            ;; difference between the two here, and it is what §B.2 note 9, §C.2
            ;; note 6 and §E.2 note 5 all turn on.
            (define name
              (if (eq? (ornament-kind found) 'emphasis-dots)
                  (list 'ornament (ornament-index found) (item-start one))
                  (list 'ornament (ornament-index found))))
            ;; §3.9.2 gives the two constructs two classes: cl-20 is "characters as
            ;; reference marks", the characters of the seal itself, and cl-21 is the
            ;; ornamented character complex a superscript or a run of emphasis dots
            ;; makes of its base.
            (struct-copy item one
                         [class (if (eq? (ornament-kind found) 'reference-mark) 20 21)]
                         [complex name]
                         [run name])])))
     (define bracketed
       (for/vector ([one (in-vector ornamented)] [index (in-naturals)])
         (define found (warichu-bracket-class one ornamented index))
         (if found (struct-copy item one [class found]) one)))
     (define separations (make-hasheqv))
     (define attachments (make-hasheqv))
     ;; §3.7.3: a jidori spreads its own run across a declared number of full-em
     ;; cells, and what the run does not fill is space between its characters.
     (for ([one (in-list (paragraph-constructs para))]
           #:when (eq? (construct-kind one) 'jidori))
       (define bases
         (for/list ([index (in-range count)]
                    #:when (let ([found (vector-ref bracketed index)])
                             (and (>= (item-start found) (construct-start one))
                                  (< (item-start found) (construct-end one)))))
           (cons index (item-advance (vector-ref bracketed index)))))
       (for ([pair (in-list (jidori-separations (cdr (assq 'cells (construct-payload one)))
                                                (extent-inline (paragraph-size para))
                                                bases
                                                style))])
         (hash-update! separations (car pair) (lambda (standing) (max standing (cdr pair))) 0)))
     (for ([one (in-list ornaments)])
       (define bases
         (for/vector ([index (in-range count)]
                      #:when (let ([found (vector-ref bracketed index)])
                               (and (>= (item-start found) (ornament-start one))
                                    (< (item-start found) (ornament-end one)))))
           (define found (vector-ref bracketed index))
           (list index (item-start found) (item-advance found) (item-size found))))
       (when (positive? (vector-length bases))
         (for ([piece (in-list (plan-ornament one bases))])
           (hash-update! attachments (attachment-anchor piece)
                         (lambda (standing) (cons piece standing)) '()))))
     (for ([one (in-list constructs)])
       (define bases
         (for/vector ([index (in-range count)]
                      #:when (let ([found (vector-ref bracketed index)])
                               (and (>= (item-start found) (ruby-start one))
                                    (< (item-start found) (ruby-end one)))))
           (list index
                 (item-start (vector-ref bracketed index))
                 (item-advance (vector-ref bracketed index)))))
       (when (positive? (vector-length bases))
         (define first-index (car (vector-ref bases 0)))
         (define last-index (car (vector-ref bases (sub1 (vector-length bases)))))
         (define em (extent-inline (ruby-em one)))
         (define found
           (plan-ruby one bases
                      (hang-before para style bracketed first-index em)
                      (hang-after para style bracketed last-index em)
                      style))
         (for ([(index amount) (in-hash (plan-separations found))])
           (hash-update! separations index (lambda (standing) (max standing amount)) 0))
         (for ([piece (in-list (plan-attachments found))])
           (hash-update! attachments (attachment-anchor piece)
                         (lambda (standing) (cons piece standing)) '()))))
     (for/vector ([one (in-vector bracketed)] [index (in-naturals)])
       (struct-copy item one
                    [separation (hash-ref separations index 0)]
                    [tail (if (= index (sub1 count)) (hash-ref separations count 0) 0)]
                    [attachments (reverse (hash-ref attachments index '()))]))]))

;; §3.3.8: how far a reading may reach back before the first base character of its
;; construct, and forward past the last.
;;
;; Two things are available: the space the neighbor's own em put at the boundary,
;; where Table 1 annotates it `hang`, and the neighbor character itself, where
;; §3.3.8's own rules allow it. Before the first item of the paragraph there is no
;; neighbor and the reading reaches into the paragraph's own indent instead, which
;; is what `ruby.overhang_indent` answers for (§B.2 note 8).
(define (hang-before para style items index em)
  (cond
    [(zero? index)
     (if (answer-is? style "ruby.overhang_indent" "prohibited")
         0
         (max 0 (paragraph-first-line-indent para)))]
    [else
     (define neighbor (vector-ref items (sub1 index)))
     (chk+ (hang-space (boundary-contributions neighbor (vector-ref items index)
                                               (paragraph-writing-mode para) style)
                       'before)
           (character-hang (item-class neighbor) (item-script para neighbor) style em))]))

(define (hang-after para style items index em)
  (cond
    [(>= (add1 index) (vector-length items)) 0]
    [else
     (define neighbor (vector-ref items (add1 index)))
     (chk+ (hang-space (boundary-contributions (vector-ref items index) neighbor
                                               (paragraph-writing-mode para) style)
                       'after)
           (character-hang (item-class neighbor) (item-script para neighbor) style em))]))

;; The part of a boundary's own space that Table 1 annotates `hang` and that the
;; NEIGHBOR's em paid for. A `hang` term measured from the ruby object's own em is
;; not a space the reading may go over: it is the object's own.
(define (hang-space terms owner)
  (for/fold ([sum 0]) ([one (in-list terms)]
                       #:when (and (contribution-hang? one) (eq? (contribution-owner one) owner)))
    (chk+ sum (contribution-amount one))))

;; The script §3.3.8 rule 2 reads, for an item that is one code point.
(define (item-script para one)
  (define text (source-slice (paragraph-source para) (item-start one) (item-end one)))
  (and (= (string-length text) 1) (script-of (char->integer (string-ref text 0)))))

;; Which boundaries a line may end at.
;;
;; A boundary the caller did not state is not an opportunity at all: §3.8.1 composes
;; "at places where line breaking is not prohibited", and the protocol's `breaks`
;; array is where the caller says which places those are for this text. Table 2 and
;; §C.3 then say which of the caller's opportunities a line may actually take.
(define (permitted-breaks para style items kinds)
  (define count (vector-length items))
  (for/hash ([index (in-range 1 count)]
             #:when (let ([kind (hash-ref kinds index #f)])
                      (or
                       ;; §3.6.3's fourth case: a tab sign that has run its stops out
                       ;; takes the rest of the line with it to the next one. The cut
                       ;; is §3.6's and not §3.1's -- it is a line boundary rather
                       ;; than a break opportunity, so it needs neither the caller's
                       ;; own `breaks` nor Table 2's permission, and it falls at
                       ;; boundaries Table 2 would never let a line end at.
                       (tab-sign? para (vector-ref items index))
                       (and kind
                            (or (eq? kind 'mandatory)
                                (breakable? para style
                                            (vector-ref items (sub1 index))
                                            (vector-ref items index)))))))
    (values index (hash-ref kinds index 'allowed))))

;; The arrangement §3.1.12's own comparison asks for: the one with the least total
;; cost, and the earliest first break among equals.
;;
;; `tail` answers, for a line starting at item `first`, what the rest of the
;; paragraph costs and where its lines end. A strict `<` is what makes the earliest
;; candidate win a tie, because the search walks the line's own end forward.
(define (choose-breaks para style items permitted kinds alignment wanted)
  (define count (vector-length items))
  (define memo (make-hash))
  (define (tail first)
    (hash-ref!
     memo
     first
     (lambda ()
       (let search ([last first] [chosen #f] [best impossible])
         (cond
           [(>= last count) (or chosen (cons impossible '()))]
           [else
            (define stop (add1 last))
            (define ends-here? (= stop count))
            (define kind (and (not ends-here?) (hash-ref permitted stop #f)))
            (define mandatory? (eq? (hash-ref kinds stop #f) 'mandatory))
            (define-values (kept kept-cost)
              (cond
                [(and (not ends-here?) (not kind)) (values chosen best)]
                [else
                 (define rest (if ends-here? (cons 0 '()) (tail stop)))
                 (define here
                   (chk+ (line-cost para style items first last ends-here? alignment
                                    (eq? kind 'discretionary) wanted)
                         (car rest)))
                 (if (< here best)
                     (values (cons here (cons (cons first last) (cdr rest))) here)
                     (values chosen best))]))
            ;; A line may not span a break the caller made mandatory, and a
            ;; mandatory break the rules refuse is still a break: the caller's own
            ;; is not an opportunity this engine may decline.
            (if mandatory? (or kept (cons impossible '())) (search stop kept kept-cost))])))))
  (cdr (tail 0)))

;; What one line costs.
(define (line-cost para style items first last last-line? alignment discretionary? wanted)
  (define found
    (measure-line para style items first last (= first 0) last-line? alignment))
  (define measure (paragraph-line-extent para))
  (define extent (shape-extent found))
  (define base
    (cond
      [(shape-cut found) impossible]
      [(positive? (shape-overrun found))
       (define over (shape-overrun found))
       (chk+ (sat* (sat* over over) overfull-weight) overfull-charge)]
      [else
       (define slack (shape-slack found))
       (define square (min (sat* slack slack) badness-cap))
       (if last-line? (div-trunc square last-line-divisor) square)]))
  (chk+ (chk+ (chk+ base (if discretionary? discretionary-charge 0))
              (chk+ (second-priority para items last last-line?)
                    (warichu-imbalance items last last-line?)))
        (if (and last-line? wanted (< (add1 (- last first)) wanted))
            widow-charge
            0)))

;; §3.4.3: what a cut between two main lines costs an inline cutting note.
;;
;; §3.4.2 divides a note into two rows "as near the same length as they can be
;; made". §3.4.3 is the same sentence one level up: a note that will not fit on one
;; line is cut between two, and the cut is as near its own middle as the caller's
;; own break opportunities allow. Each half then divides itself again, so a cut in
;; the middle gives four rows of the same length and a cut near one end gives two
;; long rows and two short ones.
;;
;; The charge is on the imbalance and it outranks the line-length terms, because the
;; section states the balance of the note rather than the balance of the lines. A
;; second half longer than the first costs a little more again, which is §3.4.2's
;; own preference read at this level.
(define (warichu-imbalance items last last-line?)
  (cond
    [last-line? 0]
    [(>= (add1 last) (vector-length items)) 0]
    [else
     (define name (warichu-of (vector-ref items last)))
     (cond
       [(not (and name (equal? name (warichu-of (vector-ref items (add1 last)))))) 0]
       [else
        (define-values (ahead behind)
          (for/fold ([ahead 0] [behind 0])
                    ([index (in-range (vector-length items))]
                     #:when (equal? name (warichu-of (vector-ref items index))))
            (if (<= index last)
                (values (chk+ ahead (item-advance (vector-ref items index))) behind)
                (values ahead (chk+ behind (item-advance (vector-ref items index)))))))
        (chk+ (chk* (sat-abs (chk- ahead behind)) warichu-balance-weight)
              (if (> behind ahead) warichu-balance-weight 0))])]))

;; §3.7.4's own Note: "In an independent formula line, when there are more than one
;; place where the line can be broken the first priority is before the math symbols
;; (cl-17), and the next is before the math operators (cl-18)." A priority among the
;; places a break may fall is a charge on the second one -- large enough that no
;; line-length argument outranks it, because the section states it as an order and
;; not as a consideration.
(define (second-priority para items last last-line?)
  (cond
    [last-line? 0]
    [(>= (add1 last) (vector-length items)) 0]
    [else
     (define after (vector-ref items (add1 last)))
     (define found (item-structure after))
     (if (and found
              (eq? (car found) 'formula)
              (cadr found)
              (eqv? (item-class after) 18))
         operator-break-charge
         0)]))

;; ----------------------------------------------------------------------------
;; Geometry
;; ----------------------------------------------------------------------------

(define (lay-out para style items breaks alignment short?)
  (define measure (paragraph-line-extent para))
  (define writing-mode (paragraph-writing-mode para))
  (define count (length breaks))
  (define block-forward? (not (eq? writing-mode 'vertical-rl)))
  (let walk ([rest breaks]
             [index 0]
             [block 0]
             [lines '()]
             [notes '()])
    (cond
      [(null? rest) (values (reverse lines) (reverse notes))]
      [else
       (define first (car (car rest)))
       (define last (cdr (car rest)))
       (define last-line? (= index (sub1 count)))
       (define found (measure-line para style items first last (= first 0) last-line? alignment))
       (define extent (shape-extent found))
       (define origin
         (case alignment
           [(end) (- measure extent)]
           [(center) (div-trunc (- measure extent) 2)]
           [else 0]))
       ;; What the line has to make room for across itself: every item's own block
       ;; size, and every annotation standing beside one.
       (define-values (spots stack-widths stack-heights)
         (warichu-layout para style items first last))
       (define block-extent
         (for/fold ([most 0]) ([offset (in-range (add1 (- last first)))])
           (define one (vector-ref items (+ first offset)))
           (define found (hash-ref spots (+ first offset) #f))
           (max most
                (cond
                  [found
                   (if (= (+ first offset) (spot-run found))
                       (max (extent-block (paragraph-size para))
                            (hash-ref stack-heights (spot-run found)))
                       0)]
                  [else
                   (chk+ (block-room one)
                         (for/fold ([beside 0]) ([found (in-list (item-attachments one))])
                           (max beside (extent-block (attachment-size found)))))]))))
       ;; The block origin is where the line starts, in both modes: horizontal
       ;; composition stacks lines downward from zero and vertical-rl stacks them
       ;; leftward from zero, so the coordinate is the running total either way and
       ;; only its sign differs.
       (define block-origin block)
       (define-values (placements marks)
         (place para style items first last found origin block-origin))
       (define one
         (line (item-start (vector-ref items first))
               (item-end (vector-ref items last))
               origin
               block-origin
               extent
               block-extent
               placements
               marks))
       (walk (cdr rest)
             (add1 index)
             (if block-forward? (+ block block-extent) (- block block-extent))
             (cons one lines)
             (append
              (if (and short? last-line?)
                  (list (list "layout.widow" "warning"
                              (item-start (vector-ref items first))
                              (item-end (vector-ref items last))
                              "3.1.9"))
                  '())
              (if (positive? (shape-overrun found))
                  (cons (list "layout.overfull" "warning"
                              (item-start (vector-ref items first))
                              (item-end (vector-ref items last))
                              "3.8.1")
                        notes)
                  notes)))])))

;; Where every cluster of one line stands, and the advance the line was composed
;; from.
(define (place para style items first last found origin block-origin)
  (define writing-mode (paragraph-writing-mode para))
  (define advances (shape-advances found))
  (define gaps (shape-gaps found))
  (define fixed (shape-fixed found))
  (define count (vector-length advances))
  (define indent (if (= first 0) (paragraph-first-line-indent para) 0))
  ;; The designed geometry: the advance a placement reports is the character's own
  ;; plus the space Table 1 states after it, before either ladder ran.
  (define-values (spots stack-widths stack-heights) (warichu-layout para style items first last))
  (define designed
    (for/vector ([offset (in-range count)])
      (define one (vector-ref items (+ first offset)))
      (define found (hash-ref spots (+ first offset) #f))
      (cond
        ;; A character of an inline cutting note reports its own advance, which is
        ;; what it was set at inside the block; the block's own width is the line's.
        [found (spot-advance found)]
        [else
         ;; §3.6: a tab sign's advance is what the line gave it and not what it was
         ;; shaped with, because taking up the space to the stop is the whole of
         ;; what the sign does.
         (define own
           (cond
             [(tab-sign? para one) (vector-ref advances offset)]
             [(collapses-at-edge? one (or (= offset 0) (= offset (sub1 count)))) 0]
             [else (item-advance one)]))
         ;; §3.4: the bracket that closes an inline cutting note is the last
         ;; character of the STRUCTURE rather than of the line, so what stands after
         ;; it stands after the whole block and is no part of the bracket's own
         ;; advance -- which is what makes the quarter em a middle dot takes after a
         ;; note visible on the line and absent from the bracket.
         (chk+ (chk+ own (if (closes-structure? one)
                             0
                             (designed-gap para style items first last offset count)))
               (vector-ref fixed (add1 offset)))])))
  (let walk ([offset 0]
             [cursor (+ origin indent (vector-ref gaps 0) (vector-ref fixed 0))]
             [out '()]
             [marks '()])
    (cond
      [(>= offset count) (values (reverse (apply append out)) (reverse (apply append marks)))]
      [else
       (define one (vector-ref items (+ first offset)))
       (walk (add1 offset)
             (chk+ (chk+ (chk+ cursor (vector-ref advances offset))
                         (vector-ref gaps (add1 offset)))
                   (vector-ref fixed (add1 offset)))
             (cons (reverse (pieces-of one cursor block-origin (vector-ref designed offset) writing-mode
                                        (hash-ref spots (+ first offset) #f)))
                   out)
             (cons (reverse (marks-of para one cursor block-origin writing-mode designed offset count))
                   marks))])))

;; The readings attached to one item, placed from that item's own position.
;;
;; §3.3.4: the reading is set above the base characters in horizontal composition
;; and to their right in vertical composition. Both are "beside the base along the
;; block axis", and the coordinate a placement reports is the box's own leading edge
;; in the direction the lines progress -- which puts the reading one of its own
;; block sizes before the base in horizontal composition and one after it in
;; vertical.
(define (marks-of para one cursor block-origin writing-mode designed offset count)
  (define vertical? (eq? writing-mode 'vertical-rl))
  (for/list ([found (in-list (item-attachments one))])
    (define size (attachment-size found))
    (define span (attachment-span found))
    ;; A ruby reading knows where it goes on its own. §3.3.9's mark and §3.7.1's
    ;; annotation are centered on what the LINE gave the base characters, so the
    ;; span is measured out of the advances the line just computed.
    (define across
      (if (zero? span)
          0
          (for/fold ([sum 0]) ([step (in-range span)]
                               #:when (< (+ offset step) count))
            (chk+ sum (vector-ref designed (+ offset step))))))
    (define start
      (if (zero? span)
          (attachment-offset found)
          (chk+ (div-trunc (chk- across (attachment-whole found)) 2) (attachment-offset found))))
    (attached (attachment-construct found)
              (attachment-start found)
              (attachment-end found)
              (chk+ cursor start)
              (if vertical?
                  (chk+ block-origin (extent-block size))
                  (chk- block-origin (extent-block size)))
              (attachment-advance found)
              size
              writing-mode
              'identity
              (attachment-symbol found))))

;; How much of the block axis one item needs.
;;
;; A cluster needs its own block size. A tate-chu-yoko run needs the whole string it
;; sets across the line, which is the sum of its members' advances and has nothing to
;; do with any one member's size.
(define (block-room one)
  (case (item-kind one)
    [(cluster tab) (extent-block (item-size one))]
    ;; A tate-chu-yoko run needs the whole string it sets across the line, which is
    ;; the sum of its members' advances and has nothing to do with any one member.
    [(tate-chu-yoko)
     (max (extent-block (item-size one))
          (for/fold ([sum 0]) ([found (in-list (item-members one))])
            (chk+ sum (piece-advance found))))]
    ;; A stacked structure needs the span its own rows cover: where each row was put
    ;; and how deep it is.
    [else
     (define blocks
       (for/list ([found (in-list (item-members one))])
         (cons (piece-block found) (extent-block (piece-size found)))))
     (cond
       [(null? blocks) (extent-block (item-size one))]
       [else
        (define top
          (for/fold ([most #f]) ([each (in-list blocks)])
            (define edge (chk+ (car each) (cdr each)))
            (if (or (not most) (> edge most)) edge most)))
        (define bottom
          (for/fold ([least #f]) ([each (in-list blocks)])
            (if (or (not least) (< (car each) least)) (car each) least)))
        (max (extent-block (item-size one)) (chk- top bottom))])]))

;; One item's placements. `designed` is the advance the answer reports for it, which
;; belongs to the item as a whole and is carried by its first member.
;;
;; §3.2.5: a run is "centered" across the line, which is the only thing the section
;; says about where its members go. The string is laid out from the middle of the
;; line outward, so a string wider than the line overhangs it on both sides equally
;; and one narrower than it sits inside it on both -- and the half of an odd width
;; that the center does not divide is taken from the leading side, which is the
;; rounding `div-trunc` already does toward zero.
(define (pieces-of one cursor block-origin designed writing-mode found)
  (define members (item-members one))
  (cond
    [found
     ;; Inside a warichu block the item stands where the block put it, on the row
     ;; the block put it on.
     (list (placed (item-index one)
                   (item-start one)
                   (item-end one)
                   (chk+ cursor (spot-inline found))
                   (chk+ block-origin (spot-block found))
                   designed
                   (item-size one)
                   (item-frame one)
                   writing-mode
                   (item-transform one)))]
    [(memq (item-kind one) '(cluster tab))
     (list (placed (item-index one)
                   (item-start one)
                   (item-end one)
                   cursor
                   block-origin
                   designed
                   (item-size one)
                   (item-frame one)
                   writing-mode
                   (item-transform one)))]
    [else
     ;; Every other kind carries its members' own two offsets, because it decided
     ;; where they go when it was built: a tate-chu-yoko run centered its string
     ;; across the line, a furawake laid its rows out one after another.
     (for/list ([found (in-list members)])
       (placed (piece-index found)
               (piece-start found)
               (piece-end found)
               (chk+ cursor (piece-inline found))
               (chk+ block-origin (piece-block found))
               (piece-advance found)
               (piece-size found)
               (piece-frame found)
               (piece-writing-mode found)
               (piece-transform found)))]))

;; The space Table 1 states after the item at `offset`, before any adjustment.
(define (designed-gap para style items first last offset count)
  (define writing-mode (paragraph-writing-mode para))
  (cond
    [(= offset (sub1 count))
     (total-of (end-contributions (vector-ref items last) style writing-mode
                                  (and (< (add1 last) (vector-length items)) (vector-ref items (add1 last)))))]
    [(after-head-space? items first count (add1 offset)) 0]
    [else
     (total-of (boundary-contributions (vector-ref items (+ first offset))
                                       (vector-ref items (+ first offset 1))
                                       writing-mode
                                       style))]))
