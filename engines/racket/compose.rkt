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
         "adjust.rkt")

(provide compose
         (struct-out placed)
         (struct-out line))

;; One cluster, placed.
(struct placed (index start end inline block advance size frame writing-mode transform) #:transparent)

;; One line of the answer.
(struct line (start end inline-origin block-origin inline-extent block-extent clusters) #:transparent)

;; ----------------------------------------------------------------------------
;; The cost constants
;; ----------------------------------------------------------------------------

;; A line that cannot be composed at all.
(define impossible infinite-cost)

;; An overfull line costs this much before its overrun is even counted, so that no
;; number of merely short lines is ever preferred to one that does not fit.
(define overfull-charge 10000000)

;; ... and its overrun counts a thousand times over, so that between two overfull
;; arrangements the one that overruns less wins.
(define overfull-weight 1000)

;; The most a single line's shortfall may contribute.
(define badness-cap 1000000)

;; §3.8.1: the last line's end "need not be aligned to the other alignment
;; position", so its shortfall is charged at a hundredth.
(define last-line-divisor 100)

;; Taking a break the caller marked discretionary is a visible act -- a hyphen
;; appears -- so it is charged even where the line it produces is perfect.
(define discretionary-charge 100000)

;; §3.5.4: a last line shorter than the caller's minimum is what widow adjustment
;; exists to avoid, so it outranks every other term.
(define widow-charge 1000000000)

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

(define (piece-of para one index transform writing-mode)
  (piece index
         (cluster-start one)
         (cluster-end one)
         (cluster-advance one)
         (cluster-size-of para one)
         (cluster-frame-of para one)
         transform
         0
         writing-mode))

(define (plain-item para style one index)
  (define transform (transform-of para one))
  (define raw
    (item index
          (cluster-start one)
          (cluster-end one)
          (cluster-advance one)
          (cluster-size-of para one)
          (cluster-frame-of para one)
          (cluster-role one)
          (classify-cluster para one style)
          transform
          'cluster
          (list (piece-of para one index transform (paragraph-writing-mode para)))))
  ;; §B.2 notes 14 through 16 and §C.2 notes 1 through 3: a class §C.3's own level
  ;; has let start a line is a different class from then on, in every table and not
  ;; only in Table 2.
  (struct-copy item raw [class (reclassified para style raw (item-class raw))]))

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
  (define members
    (for/list ([one (in-list clusters)] [at (in-list indices)])
      (piece-of para one at 'tate-chu-yoko 'horizontal-tb)))
  (define across
    (for/fold ([sum 0]) ([one (in-list members)]) (chk+ sum (piece-advance one))))
  (define along
    (for/fold ([most 0]) ([one (in-list members)])
      (max most (extent-block (piece-size one)))))
  (item index
        (cluster-start (car clusters))
        (cluster-end (last clusters))
        along
        (extent along across)
        (cluster-frame-of para (car clusters))
        #f
        30
        'tate-chu-yoko
        'tate-chu-yoko
        members))

;; The things that stand on the line: the caller's clusters, with each construct's
;; own clusters gathered into the one item that construct is.
(define (items-of para style)
  (define clusters (paragraph-clusters para))
  (define count (vector-length clusters))
  (define runs (grouping-constructs para))
  (define out '())
  (let walk ([index 0] [next 0])
    (cond
      [(>= index count) (void)]
      [else
       (define found (assv (cluster-start (vector-ref clusters index)) runs))
       (cond
         [found
          (define stop (cdr found))
          (define inside
            (for/list ([at (in-range index count)]
                       #:break (>= (cluster-start (vector-ref clusters at)) stop))
              at))
          (set! out (cons (run-item para
                                    (for/list ([at (in-list inside)]) (vector-ref clusters at))
                                    inside
                                    next)
                          out))
          (walk (+ index (length inside)) (add1 next))]
         [else
          (set! out (cons (plain-item para style (vector-ref clusters index) index) out))
          (walk (add1 index) (add1 next))])]))
  (list->vector (reverse out)))

;; The constructs that gather clusters into one item, as `(start . end)` pairs.
(define (grouping-constructs para)
  (for/list ([one (in-list (paragraph-constructs para))]
             #:when (eq? (construct-kind one) 'tate-chu-yoko))
    (cons (construct-start one) (construct-end one))))

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
;; One line, measured
;; ----------------------------------------------------------------------------

;; A line as the ladders see it: the advance of every item, the amount at every
;; boundary and at the two edges, and what may be given back or taken up where.
(struct shape (advances gaps extent overrun hung) #:transparent)

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
  (define advances
    (for/vector ([offset (in-range count)])
      (define one (vector-ref items (+ first offset)))
      (if (collapses-at-edge? one (or (= offset 0) (= offset (sub1 count)))) 0 (item-advance one))))
  (define gap-terms
    (for/vector ([index (in-range (add1 count))])
      (cond
        [(= index 0) (head-contributions (vector-ref items first) style first-line?)]
        [(= index count) (end-contributions (vector-ref items last) style)]
        [else
         (boundary-contributions (vector-ref items (+ first index -1))
                                 (vector-ref items (+ first index))
                                 writing-mode
                                 style)])))
  (define gaps (vector-map total-of gap-terms))
  (define natural
    (+ indent
       (for/fold ([sum 0]) ([one (in-vector advances)]) (chk+ sum one))
       (for/fold ([sum 0]) ([one (in-vector gaps)]) (chk+ sum one))))
  (define measure (paragraph-line-extent para))
  (define room (- measure natural))
  (cond
    [(negative? room)
     (define-values (kept-advances kept-gaps left)
       (reduce para style items first last advances gaps (- room)))
     (define reduced (+ indent (sum-of kept-advances) (sum-of kept-gaps)))
     (define hung (hang-of para style items last (- reduced measure)))
     (shape kept-advances kept-gaps (- reduced hung) (max 0 (- reduced hung measure)) hung)]
    [(and (positive? room) (eq? alignment 'justify) (not last-line?))
     (define-values (open-advances open-gaps)
       (expand para style items first last advances gaps room))
     (shape open-advances open-gaps (+ indent (sum-of open-advances) (sum-of open-gaps)) 0 0)]
    [else (shape advances gaps natural 0 0)]))

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

;; One place a line may give space back.
(struct opening (where index floor stage two-valued?) #:transparent)

(define (reduction-sites para style items first last advances gaps)
  (define count (vector-length advances))
  (define table (reduction-table-of style))
  (append
   ;; §3.8.3 (a) and §D's own first stage: the Western word spaces, all at once.
   (for/list ([offset (in-range count)]
              #:when (and (word-space? (vector-ref items (+ first offset)))
                          (> (vector-ref advances offset) 0)))
     (opening 'advance offset (word-space-floor (item-em (vector-ref items (+ first offset)))) 1 #f))
   ;; Tables 3 through 5, which are "the second and subsequent stages". The line
   ;; head is not among them: all three tables prohibit reduction there.
   (for/list ([index (in-range 1 (add1 count))]
              #:when (and (> (vector-ref gaps index) 0)
                          (or (= index count) (not (collapsed-here? items first count index)))))
     (define amount (vector-ref gaps index))
     (define found
       (if (= index count)
           (reduction-of table (item-class (vector-ref items last)) line-edge amount)
           (reduction-of table
                         (item-class (vector-ref items (+ first index -1)))
                         (item-class (vector-ref items (+ first index)))
                         amount)))
     (opening 'gap index (reduction-floor found) (reduction-stage found) (reduction-two-valued? found)))))

;; Give back `wanted`, in stage order, and report what is left.
(define (reduce para style items first last advances gaps wanted)
  (define kept-advances (vector-copy advances))
  (define kept-gaps (vector-copy gaps))
  (define sites (filter opening-stage (reduction-sites para style items first last advances gaps)))
  (define trailing? (answer-is? style "adjustment.remainder" "trailing"))
  (define left
    (for/fold ([left wanted]) ([stage (in-range 1 (add1 tables:max-stage))])
      (define here (filter (lambda (one) (eqv? (opening-stage one) stage)) sites))
      (cond
        [(or (<= left 0) (null? here)) left]
        [else (take-back kept-advances kept-gaps here left trailing?)])))
  (values kept-advances kept-gaps left))

(define (site-current advances gaps one)
  (if (eq? (opening-where one) 'advance)
      (vector-ref advances (opening-index one))
      (vector-ref gaps (opening-index one))))

(define (site-set! advances gaps one value)
  (if (eq? (opening-where one) 'advance)
      (vector-set! advances (opening-index one) value)
      (vector-set! gaps (opening-index one) value)))

;; Take `wanted` out of one stage's sites, and report what could not be taken.
;;
;; §3.1.9's two-valued sites are the exception to "equally": a half em after a
;; closing bracket at the line end is a half em or it is nothing, and no size in
;; between, so such a site is closed outright as soon as the line needs anything at
;; all -- which can leave the line shorter than the measure asked for, and does.
(define (take-back advances gaps sites wanted trailing?)
  (define-values (jumps rest) (partition opening-two-valued? sites))
  (define after-jumps
    (for/fold ([left wanted]) ([one (in-list jumps)])
      (cond
        [(<= left 0) left]
        [else
         (define room (- (site-current advances gaps one) (opening-floor one)))
         (site-set! advances gaps one (opening-floor one))
         (- left room)])))
  (let spread ([left after-jumps] [open rest])
    (cond
      [(<= left 0) 0]
      [(null? open) left]
      [else
       (define count (length open))
       (define share (div-trunc left count))
       (define extra (- left (* share count)))
       (define ordered (if trailing? (reverse open) open))
       (define taken
         (for/fold ([taken 0]) ([one (in-list ordered)] [rank (in-naturals)])
           (define want (+ share (if (< rank extra) 1 0)))
           (define room (- (site-current advances gaps one) (opening-floor one)))
           (define now (min want room))
           (site-set! advances gaps one (- (site-current advances gaps one) now))
           (+ taken now)))
       (define remaining
         (filter (lambda (one) (> (- (site-current advances gaps one) (opening-floor one)) 0)) open))
       (cond
         [(zero? taken) (- left 0)]
         [else (spread (- left taken) remaining)])])))

;; ----------------------------------------------------------------------------
;; §3.8.4, the expansion ladder
;; ----------------------------------------------------------------------------

;; One place a line may take space up. `ceiling` is #f where step (d) is the only
;; thing that reaches it, which is unbounded.
(struct widening (where index ceiling stage) #:transparent)

(define (expansion-sites para style items first last advances gaps)
  (define count (vector-length advances))
  (append
   (for/list ([offset (in-range count)]
              #:when (and (word-space? (vector-ref items (+ first offset)))
                          (> (vector-ref advances offset) 0)))
     (widening 'advance offset (word-space-ceiling (item-em (vector-ref items (+ first offset)))) 1))
   (append*
    (for/list ([index (in-range 1 count)])
      (define before (vector-ref items (+ first index -1)))
      (define after (vector-ref items (+ first index)))
      (define found (expansion-of (item-class before) (item-class after) (item-em after)))
      (cond
        ;; §B.2 note 13: a word space the line edge collapsed is not on the line,
        ;; and a boundary beside something that is not there is not a place to put
        ;; space.
        [(collapsed-here? items first count index) '()]
        ;; §E.2 note 10: no expansion between a quantity symbol or a European
        ;; numeral and the postfixed abbreviation (cl-13) it holds on to.
        [(and (= (item-class before) 27)
              (= (item-class after) 13)
              (quantity-or-numeral? para before))
         '()]
        [else
         (append (if (expansion-stage found)
                     (list (widening 'gap index (expansion-ceiling found) (expansion-stage found)))
                     '())
                 (if (expansion-residual? found)
                     (list (widening 'gap index #f 4))
                     '()))])))))

;; Whether the boundary before offset `index` touches a word space the line edge
;; collapsed. §3.2.2's space is restored the moment the same text sits elsewhere on
;; a line, so this is a fact about the line and not about the cluster.
(define (collapsed-here? items first count index)
  (define (collapsed? offset)
    (and (or (= offset 0) (= offset (sub1 count)))
         (word-space? (vector-ref items (+ first offset)))))
  (or (collapsed? (sub1 index)) (collapsed? index)))

;; Take up `room`, in stage order.
(define (expand para style items first last advances gaps room)
  (define open-advances (vector-copy advances))
  (define open-gaps (vector-copy gaps))
  (define sites (expansion-sites para style items first last advances gaps))
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
       (define count (length open))
       (define share (div-trunc left count))
       (define extra (- left (* share count)))
       (define ordered (if trailing? (reverse open) open))
       (define given
         (for/fold ([given 0]) ([one (in-list ordered)] [rank (in-naturals)])
           (define want (+ share (if (< rank extra) 1 0)))
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
  (define items (items-of para style))
  (define count (vector-length items))
  (cond
    [(zero? count) (values '() '())]
    [else
     (define kinds (break-kinds para items))
     (define alignment (or (paragraph-alignment para) 'justify))
     (define permitted (permitted-breaks para style items kinds))
     (define breaks (choose-breaks para style items permitted kinds alignment))
     (lay-out para style items breaks alignment)]))

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
                      (and kind
                           (or (eq? kind 'mandatory)
                               (breakable? para style
                                           (vector-ref items (sub1 index))
                                           (vector-ref items index))))))
    (values index (hash-ref kinds index))))

;; The arrangement §3.1.12's own comparison asks for: the one with the least total
;; cost, and the earliest first break among equals.
;;
;; `tail` answers, for a line starting at item `first`, what the rest of the
;; paragraph costs and where its lines end. A strict `<` is what makes the earliest
;; candidate win a tie, because the search walks the line's own end forward.
(define (choose-breaks para style items permitted kinds alignment)
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
                                    (eq? kind 'discretionary))
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
(define (line-cost para style items first last last-line? alignment discretionary?)
  (define found
    (measure-line para style items first last (= first 0) last-line? alignment))
  (define measure (paragraph-line-extent para))
  (define extent (shape-extent found))
  (define base
    (cond
      [(> extent measure)
       (define over (- extent measure))
       (chk+ (sat* (sat* over over) overfull-weight) overfull-charge)]
      [else
       (define slack (- measure extent))
       (define square (min (sat* slack slack) badness-cap))
       (if last-line? (div-trunc square last-line-divisor) square)]))
  (chk+ (chk+ base (if discretionary? discretionary-charge 0))
        (if (and last-line?
                 (paragraph-widow-minimum para)
                 (< (add1 (- last first)) (paragraph-widow-minimum para)))
            widow-charge
            0)))

;; ----------------------------------------------------------------------------
;; Geometry
;; ----------------------------------------------------------------------------

(define (lay-out para style items breaks alignment)
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
       (define block-extent
         (for/fold ([most 0]) ([offset (in-range (add1 (- last first)))])
           (max most (block-room (vector-ref items (+ first offset))))))
       ;; The block origin is where the line starts, in both modes: horizontal
       ;; composition stacks lines downward from zero and vertical-rl stacks them
       ;; leftward from zero, so the coordinate is the running total either way and
       ;; only its sign differs.
       (define block-origin block)
       (define placements
         (place para style items first last found origin block-origin))
       (define one
         (line (item-start (vector-ref items first))
               (item-end (vector-ref items last))
               origin
               block-origin
               extent
               block-extent
               placements))
       (walk (cdr rest)
             (add1 index)
             (if block-forward? (+ block block-extent) (- block block-extent))
             (cons one lines)
             (if (positive? (shape-overrun found))
                 (cons (list "layout.overfull" "warning"
                             (item-start (vector-ref items first))
                             (item-end (vector-ref items last))
                             "3.8.1")
                       notes)
                 notes))])))

;; Where every cluster of one line stands, and the advance the line was composed
;; from.
(define (place para style items first last found origin block-origin)
  (define writing-mode (paragraph-writing-mode para))
  (define advances (shape-advances found))
  (define gaps (shape-gaps found))
  (define count (vector-length advances))
  (define indent (if (= first 0) (paragraph-first-line-indent para) 0))
  ;; The designed geometry: the advance a placement reports is the character's own
  ;; plus the space Table 1 states after it, before either ladder ran.
  (define designed
    (for/vector ([offset (in-range count)])
      (define one (vector-ref items (+ first offset)))
      (define own
        (if (collapses-at-edge? one (or (= offset 0) (= offset (sub1 count)))) 0 (item-advance one)))
      (chk+ own (designed-gap para style items first last offset count))))
  (let walk ([offset 0] [cursor (+ origin indent (vector-ref gaps 0))] [out '()])
    (cond
      [(>= offset count) (reverse (apply append out))]
      [else
       (define one (vector-ref items (+ first offset)))
       (walk (add1 offset)
             (chk+ (chk+ cursor (vector-ref advances offset)) (vector-ref gaps (add1 offset)))
             (cons (reverse (pieces-of one cursor block-origin (vector-ref designed offset) writing-mode))
                   out))])))

;; How much of the block axis one item needs.
;;
;; A cluster needs its own block size. A tate-chu-yoko run needs the whole string it
;; sets across the line, which is the sum of its members' advances and has nothing to
;; do with any one member's size.
(define (block-room one)
  (if (eq? (item-kind one) 'cluster)
      (extent-block (item-size one))
      (max (extent-block (item-size one))
           (for/fold ([sum 0]) ([found (in-list (item-members one))])
             (chk+ sum (piece-advance found))))))

;; One item's placements. `designed` is the advance the answer reports for it, which
;; belongs to the item as a whole and is carried by its first member.
;;
;; §3.2.5: a run is "centered" across the line, which is the only thing the section
;; says about where its members go. The string is laid out from the middle of the
;; line outward, so a string wider than the line overhangs it on both sides equally
;; and one narrower than it sits inside it on both -- and the half of an odd width
;; that the center does not divide is taken from the leading side, which is the
;; rounding `div-trunc` already does toward zero.
(define (pieces-of one cursor block-origin designed writing-mode)
  (define members (item-members one))
  (cond
    [(eq? (item-kind one) 'cluster)
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
     (define across
       (for/fold ([sum 0]) ([found (in-list members)]) (chk+ sum (piece-advance found))))
     (let walk ([rest members] [block (chk- block-origin (div-trunc across 2))] [out '()])
       (cond
         [(null? rest) (reverse out)]
         [else
          (define found (car rest))
          (walk (cdr rest)
                (chk+ block (piece-advance found))
                (cons (placed (piece-index found)
                              (piece-start found)
                              (piece-end found)
                              cursor
                              block
                              (piece-advance found)
                              (piece-size found)
                              (piece-frame found)
                              (piece-writing-mode found)
                              (piece-transform found))
                      out))]))]))

;; The space Table 1 states after the item at `offset`, before any adjustment.
(define (designed-gap para style items first last offset count)
  (define writing-mode (paragraph-writing-mode para))
  (cond
    [(= offset (sub1 count)) (total-of (end-contributions (vector-ref items last) style))]
    [else
     (total-of (boundary-contributions (vector-ref items (+ first offset))
                                       (vector-ref items (+ first offset 1))
                                       writing-mode
                                       style))]))
