#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; §3.3 and Appendix F: ruby.
;;
;; A ruby construct is a second stream of text set beside the first. The base
;; characters stay where they are -- each is still its own item on the line, and
;; §3.9.2 gives it cl-22 or cl-23 rather than the class its own code point would
;; have -- and the reading is an ATTACHMENT: it has a position on the line and no
;; advance of its own. What ruby can do to the line is push the base characters
;; apart, and that is a SEPARATION: an amount of space forced in at a boundary
;; because the reading had nowhere else to go.
;;
;; Three layouts, and which one a run gets
;;
;;   per-run     §3.3.5. One reading over one base character: centered on it
;;               (nakatsuki) or started with it (katatsuki), hanging over what
;;               stands beside it as far as §3.3.8 allows and forcing a separation
;;               for the rest. Mono ruby is always this; so is a group run over one
;;               base character, because §3.3.6's two methods are both stated over
;;               "the inter-character spacing between each adjacent base character"
;;               and a run over one base character has no adjacent base character.
;;   distribute  §3.3.6. A group run over two or more base characters: whichever of
;;               the reading and the base is shorter is spread across the other,
;;               by `ruby.group_distribution`'s two methods.
;;   compound    §F. A jukugo compound one of whose runs carries three or more ruby
;;               characters (§F.1: two or fewer stay with their base). The whole
;;               reading is set solid across the whole compound and the base
;;               characters are pushed apart under it.
;;
;; §F.3's own formula
;;
;; "Total inter-character spacing = (the sum of the length of those ruby characters
;; forced out from the corresponding base character) - (the sum ... which overhang
;; other base characters) - (the sum ... which overhang other non-base characters)".
;; The second and third terms are facts about a compound whose base characters have
;; already been pushed apart by the total being computed, so the formula refers to
;; its own result. What this engine computes instead is the least total at which the
;; whole reading has somewhere to stand: the reading is as long as it is, the base
;; is as long as it is, and what the two ends may hang over is what §3.3.8 allows —
;; so the total is the reading's own excess less what the two ends absorb, and it is
;; arrived at directly rather than by iterating the sentence.
;;
;; Then §F.4: the total is shared out "proportional to reading length", over the
;; boundaries of the runs that HAVE an excess. A run whose reading is no longer than
;; its own base character asks for nothing and its two boundaries carry none of the
;; total, which is what makes a four-ruby run expand only the base it is over.

(require racket/list
         "arith.rkt"
         "model.rkt"
         "style.rkt"
         "classes.rkt"
         "spacing.rkt"
         (prefix-in tables: "tables.rkt"))

(provide (struct-out annotation)
         (struct-out run)
         (struct-out ruby)
         ruby-constructs
         ruby-class-of
         plan-ruby
         character-hang
         (struct-out plan))

;; ----------------------------------------------------------------------------
;; The construct, read out of the request
;; ----------------------------------------------------------------------------

;; One cluster of the reading.
(struct annotation (start end advance size) #:transparent)

;; One run: the base clusters it is set over and the reading set over them.
(struct run (base-start base-end annotations) #:transparent)

;; One ruby construct.
(struct ruby (index kind start end em runs) #:transparent)

(define (field what object name)
  (define found (hash-ref object name #f))
  (unless found
    (fail-input "~a does not state ~a" what name))
  found)

;; The annotation stream of one construct, as clusters.
(define (read-annotation value)
  (define what "a ruby annotation")
  (unless (hash? value)
    (fail-input "~a is not a JSON object" what))
  (define size (field what value 'size))
  (define default
    (extent (hash-ref size 'inline (lambda () (fail-input "~a states no inline size" what)))
            (hash-ref size 'block (lambda () (fail-input "~a states no block size" what)))))
  (values default
          (for/list ([one (in-list (field what value 'clusters))])
            (define range (field what one 'range))
            (define own (hash-ref one 'size #f))
            (annotation (car range)
                        (cadr range)
                        (field what one 'advance)
                        (if own (extent (hash-ref own 'inline) (hash-ref own 'block)) default)))))

;; Every ruby construct of the request, in the order the caller stated them.
(define (ruby-constructs para)
  (for/list ([one (in-list (paragraph-constructs para))]
             [index (in-naturals)]
             #:when (eq? (construct-kind one) 'ruby))
    (define payload (construct-payload one))
    (define kind (string->symbol (cdr (assq 'ruby-kind payload))))
    (define-values (em clusters) (read-annotation (cdr (assq 'annotation payload))))
    (define by-range
      (for/list ([found (in-list (cdr (assq 'runs payload)))])
        (define base (field "a ruby run" found 'base))
        (define reading (field "a ruby run" found 'annotation))
        (run (car base)
             (cadr base)
             (for/list ([piece (in-list clusters)]
                        #:when (and (>= (annotation-start piece) (car reading))
                                    (<= (annotation-end piece) (cadr reading))))
               piece))))
    (ruby index kind (construct-start one) (construct-end one) em by-range)))

;; §3.9.2: a base character of a ruby construct is a member of the complex rather
;; than of its own code point's class. Jukugo ruby has a class of its own.
(define (ruby-class-of one)
  (if (eq? (ruby-kind one) 'jukugo) 23 22))

;; ----------------------------------------------------------------------------
;; The plan
;; ----------------------------------------------------------------------------

;; What one construct does to the line.
;;
;; `separations` maps an item to the space forced in BEFORE it and `tails` to the
;; space forced in AFTER it. They are two hashes and not one because a run can end a
;; line: the space its reading needs after it is then at the line end AND at the head
;; of the next line, and a boundary keyed only by the item after it would lose one of
;; the two. `attachments` is where the reading goes.
(struct plan (separations tails attachments) #:transparent)

;; A share of `total` for each weight, with the remainder going to the leading
;; shares or the trailing ones as `adjustment.remainder` answers.
(define (shares total weights style)
  (define sum (for/fold ([sum 0]) ([one (in-list weights)]) (+ sum one)))
  (cond
    [(or (<= total 0) (<= sum 0)) (map (lambda (one) 0) weights)]
    [else
     (define floors (for/list ([one (in-list weights)]) (div-trunc (chk* total one) sum)))
     (define left (chk- total (for/fold ([sum 0]) ([one (in-list floors)]) (chk+ sum one))))
     (define trailing? (answer-is? style "adjustment.remainder" "trailing"))
     (define count (length floors))
     ;; The leftover units go one to a share, from whichever end the answer names,
     ;; and only to shares that were entitled to something in the first place.
     (define eligible
       (for/list ([one (in-list weights)] [index (in-naturals)] #:when (> one 0)) index))
     (define taking
       (let loop ([rest (if trailing? (reverse eligible) eligible)] [left left] [out '()])
         (cond
           [(or (<= left 0) (null? rest)) out]
           [else (loop (cdr rest) (sub1 left) (cons (car rest) out))])))
     (for/list ([one (in-list floors)] [index (in-naturals)])
       (if (memv index taking) (add1 one) one))]))

;; §3.3.6's own two methods, as a weight vector over `count + 1` gaps.
;;
;; `jis` is the ratio the section states -- "1 unit of spacing between the start of
;; the base text and the start of the ruby text, and between the end of the ruby
;; text and the end of the base text", with two units between every interior pair --
;; so the vector is 1, 2, ..., 2, 1. `flush` aligns the two leading characters and
;; the two trailing ones and spaces out "the rest", so the two end gaps are zero.
(define (distribution-weights count flush?)
  (cond
    [(<= count 1) (if flush? (list 0 0) (list 1 1))]
    [else
     (define interior (for/list ([index (in-range (sub1 count))]) 2))
     (if flush?
         (append (list 0) (map (lambda (one) 1) interior) (list 0))
         (append (list 1) interior (list 1)))]))

;; The reading laid out along the base, or the base along the reading.
(define (spread total widths weights style)
  (define gaps (shares total weights style))
  (let walk ([rest widths] [open (cdr gaps)] [at (car gaps)] [out '()])
    (cond
      [(null? rest) (values (reverse out) gaps)]
      [else
       (walk (cdr rest) (cdr open) (chk+ (chk+ at (car rest)) (car open)) (cons at out))])))

;; ----------------------------------------------------------------------------
;; §3.3.8: what a reading may hang over
;; ----------------------------------------------------------------------------

;; "The full-width size of the ruby characters", which every one of §3.3.8's
;; allowances and §F.1's own overhang are stated in.
;;
;; Two things a reading of one size cannot say, and one whose characters differ can.
;; The size is the largest of the run's own characters and not the size the construct
;; declared for its annotation -- the declared size is a default for a cluster that
;; states none rather than an answer of its own -- and it belongs to the RUN whose
;; reading is doing the overhanging, because that is the subject of every sentence
;; that states an allowance: "a run of ruby text for a given base character is
;; allowed to overhang" is a fact about that run's characters and not about a
;; character somewhere else in the compound.
(define (run-unit piece)
  (for/fold ([widest 0]) ([found (in-list (run-annotations piece))])
    (max widest (extent-inline (annotation-size found)))))

;; How far a reading may reach over the neighbor on one side of its construct.
;;
;; §3.3.8 states seven allowances and they are not one rule with exceptions: three
;; name a character the reading may go over, one names a space it may go over, one
;; names a mixture of the two, and the Notes' variations reach only the first. `side`
;; is which side of the construct the neighbor stands on, because three of the seven
;; are stated for one side and not the other. `space` is how much of the amount
;; Table 1 states at the boundary the NEIGHBOR's own em paid for; `unit` is the
;; full-width size of the ruby characters.
;;
;; `neighbor-script` is what `spec/derived/scripts.tsv` names the character, as a
;; string, or #f where the item is not one code point. §3.3.8's second allowance
;; names "hiragana (cl-15), katakana (cl-16), prolonged sound mark (cl-10) or small
;; kana (cl-11)", which is two scripts and two classes spelled in them, and the two
;; readings part at exactly the marks they disagree about -- U+30FC is cl-10 and
;; Script=Common, and the katakana iteration marks are cl-09 and Script=Katakana
;; (docs/decisions/ruby-overhang-permission.md).
(define (character-hang neighbor-class neighbor-script side style unit space)
  (define chosen (answer style "ruby.overhang_kana"))
  (cond
    ;; A middle dot (cl-05), whose allowance is the one §3.3.8 states as a sum: the
    ;; spacing on the far side of the dot plus half a ruby character. The section
    ;; states it for a dot whose spacing "is reduced ... as a result of the line
    ;; adjustment" and states one ruby character otherwise; the sum is the smaller of
    ;; the two wherever the spacing stands unreduced, so taking it always is taking
    ;; the section's own answer at every coordinate it states one for.
    [(= neighbor-class 5) (min unit (chk+ space (div-trunc unit 2)))]
    ;; An inseparable character (cl-08), and §B.2 note 8's ideographic space (cl-14).
    [(memv neighbor-class '(8 14)) unit]
    ;; §3.3.8's two bracket allowances, each stated for one side only. An opening
    ;; bracket BEFORE the object and a closing bracket, full stop or comma AFTER it
    ;; are characters the reading may go over; on the other side, what the section
    ;; offers is the half em that stands between them and the object, and no more.
    [(= neighbor-class 1) (if (eq? side 'before) unit (min unit space))]
    [(memv neighbor-class '(2 6 7)) (if (eq? side 'after) unit (min unit space))]
    [(string=? chosen "any") unit]
    [(string=? chosen "none") 0]
    [(equal? neighbor-script "Hiragana") unit]
    [(and (equal? neighbor-script "Katakana") (not (string=? chosen "jis"))) unit]
    [else 0]))

;; ----------------------------------------------------------------------------
;; Planning one construct
;; ----------------------------------------------------------------------------

;; `bases` is a vector of `(index start advance)` triples, one per base item of the
;; construct, in line order -- the item's own place in the paragraph, the byte offset
;; its text starts at, and the advance it was shaped with. `lead` and `trail` answer
;; how far the reading may hang before the first base character and after the last;
;; each takes the full-width size of the ruby characters that would do the hanging,
;; because §3.3.8 states every one of its allowances in that size and the run at one
;; end of a compound need not be set at the size of the run at the other.
(define (plan-ruby one bases lead trail style)
  (define compound?
    (and (eq? (ruby-kind one) 'jukugo)
         (for/or ([piece (in-list (ruby-runs one))]) (>= (length (run-annotations piece)) 3))))
  (cond
    ;; §3.3.7 names two methods for a jukugo compound one of whose runs is too long
    ;; to stay with its base: the one JIS X 4051 specifies, which is §3.3.6's own
    ;; group method applied to the whole compound, and the one decided by the
    ;; phonetic structure, which is Appendix F's. `ruby.jukugo_layout` chooses.
    ;; §3.3.7 names its first method by author -- "the method specified in JIS X
    ;; 4051" -- and not by §3.3.6's own choice, so `ruby.group_distribution` selects
    ;; nothing here: the ratio is the one that section states, and the flush method
    ;; is the other reading of §3.3.6 rather than another reading of §3.3.7.
    [(and compound? (answer-is? style "ruby.jukugo_layout" "group"))
     (plan-distributed one bases (whole-reading one) style #f)]
    [compound? (plan-compound one bases lead trail style)]
    [(and (eq? (ruby-kind one) 'group) (> (vector-length bases) 1))
     (plan-distributed one bases (run-annotations (car (ruby-runs one))) style
                       (answer-is? style "ruby.group_distribution" "flush"))]
    [else (plan-per-run one bases lead trail style)]))

;; Every reading cluster of the construct, in order.
(define (whole-reading one)
  (append* (for/list ([piece (in-list (ruby-runs one))]) (run-annotations piece))))

(define (total-of widths)
  (for/fold ([sum 0]) ([one (in-list widths)]) (chk+ sum one)))

(define (reading-width piece)
  (total-of (for/list ([found (in-list (run-annotations piece))]) (annotation-advance found))))

;; The base items one run covers. A run names its base by BYTE RANGE, which is what
;; the caller stated it in; the item's own index is what a separation is keyed by.
(define (base-index found) (first found))
(define (base-start found) (second found))
(define (base-advance found) (third found))

(define (run-bases piece bases)
  (for/list ([found (in-vector bases)]
             #:when (and (>= (base-start found) (run-base-start piece))
                         (< (base-start found) (run-base-end piece))))
    found))

;; ----------------------------------------------------------------------------
;; §3.3.5: one reading over one base character
;; ----------------------------------------------------------------------------

(define (plan-per-run one bases lead trail style)
  (define katatsuki? (answer-is? style "ruby.alignment" "katatsuki"))
  (define separations (make-hash))
  (define tails (make-hash))
  (define attachments '())
  (for ([piece (in-list (ruby-runs one))] [rank (in-naturals)])
    (define here (run-bases piece bases))
    (when (pair? here)
      (define anchor (base-index (car here)))
      (define width (total-of (map base-advance here)))
      (define reading (reading-width piece))
      (define overflow (max 0 (chk- reading width)))
      ;; §3.3.5's own centering takes the lower half of an odd difference. The space
      ;; that centering then forces is two adjustment sites and takes
      ;; `adjustment.remainder`'s own order, so the two roundings are not the same
      ;; number and are not computed from each other.
      ;; §3.3.5(b): katatsuki starts the reading with its base character. Its own
      ;; text for the three-or-more case states two methods, and the second -- which
      ;; way the overflow leans -- is a choice among overhangs onto the adjacent
      ;; characters. Where the reading is longer than its base there is no choice
      ;; left to make that is not an overhang, and what remains is (b)(i): the same
      ;; centering nakatsuki states. So the two alignments part only where the
      ;; reading FITS (docs/decisions/mono-ruby-separation-split.md).
      (define offset
        (cond
          [(positive? overflow) (- (div-trunc overflow 2))]
          [katatsuki? 0]
          [else (div-trunc (chk- width reading) 2)]))
      (define split (shares overflow (list 1 1) style))
      (define first? (zero? rank))
      (define last? (= rank (sub1 (length (ruby-runs one)))))
      ;; A run's own excess may hang over what stands outside the construct; inside
      ;; it, the next base character is where the neighboring run's own reading is,
      ;; so the excess is forced rather than hung.
      (define unit (run-unit piece))
      (define before (max 0 (chk- (first split) (if first? (lead unit) 0))))
      (define after (max 0 (chk- (second split) (if last? (trail unit) 0))))
      (when (positive? before)
        (hash-update! separations anchor (lambda (found) (max found before)) 0))
      (when (positive? after)
        (hash-update! tails (base-index (last here)) (lambda (found) (max found after)) 0))
      (let walk ([rest (run-annotations piece)] [at offset])
        (unless (null? rest)
          (define found (car rest))
          (set! attachments
                (cons (attachment (ruby-index one) anchor 0 0 at
                                  (annotation-start found) (annotation-end found)
                                  (annotation-advance found) (annotation-size found) #f)
                      attachments))
          (walk (cdr rest) (chk+ at (annotation-advance found)))))))
  (plan separations tails (reverse attachments)))

;; ----------------------------------------------------------------------------
;; §3.3.6: a group run over two or more base characters
;; ----------------------------------------------------------------------------

(define (plan-distributed one bases readings style flush?)
  (define widths (for/list ([found (in-list readings)]) (annotation-advance found)))
  (define reading (total-of widths))
  (define base-widths (for/list ([found (in-vector bases)]) (base-advance found)))
  (define width (total-of base-widths))
  (define anchor (base-index (vector-ref bases 0)))
  (cond
    ;; The reading is the shorter one: spread it across the base.
    [(<= reading width)
     (define-values (offsets gaps)
       (spread (chk- width reading) widths (distribution-weights (length widths) flush?) style))
     (plan (make-hash) (make-hash)
           (for/list ([found (in-list readings)] [at (in-list offsets)])
             (attachment (ruby-index one) anchor 0 0 at
                         (annotation-start found) (annotation-end found)
                         (annotation-advance found) (annotation-size found) #f)))]
    ;; The base is the shorter one: spread IT across the reading, which is what
    ;; pushes the base characters apart and is the construct's own separation.
    [else
     (define-values (offsets gaps)
       (spread (chk- reading width) base-widths
               (distribution-weights (vector-length bases) flush?) style))
     (define separations (make-hash))
     (define tails (make-hash))
     (for ([found (in-vector bases)] [amount (in-list gaps)])
       (when (positive? amount)
         (hash-set! separations (base-index found) amount)))
     (define final (last gaps))
     (when (positive? final)
       (hash-set! tails (base-index (vector-ref bases (sub1 (vector-length bases)))) final))
     (plan separations tails
           (let walk ([rest readings] [at (- (car gaps))] [out '()])
             (cond
               [(null? rest) (reverse out)]
               [else
                (define found (car rest))
                (walk (cdr rest)
                      (chk+ at (annotation-advance found))
                      (cons (attachment (ruby-index one) anchor 0 0 at
                                        (annotation-start found) (annotation-end found)
                                        (annotation-advance found) (annotation-size found) #f)
                            out))])))]))

;; ----------------------------------------------------------------------------
;; §F: the whole compound at once
;; ----------------------------------------------------------------------------

;; §F.2 and §F.3, in the order §F.2 states them: let each run reach over the base
;; character beside it, then over what stands outside the compound, and only then
;; open the compound up.
;;
;; §F.2's own order is what makes the arrangement asymmetric. "Let a run of ruby
;; text for a given base character overhang either or both of the adjacent base
;; characters up to a maximum of one em in the ruby character size. THE FIRST CHOICE
;; SHOULD BE THE SUCCEEDING BASE CHARACTER" -- and, where no arrangement following
;; that choice exists, "let them overhang the preceding base characters". So a run
;; goes as far forward as its own excess needs and no further, unless a later run
;; leaves it nowhere to be, in which case it moves back.
;;
;; That is two passes and not a search. `latest` walks the runs from the last
;; backwards and is the furthest forward each run may stand once every run after it
;; has somewhere to go; `natural` is where §F.2's first choice puts a run reading
;; nothing else; and a run stands at the later of its natural place and the end of
;; the run before it, brought back to `latest` where that is earlier.

;; Where one run's base characters stand, relative to the first base character of the
;; construct, once `gaps` has been forced in: the start of each run's base text and
;; the end of it.
(define (run-extents widths gaps)
  (let walk ([rest widths] [open (cdr gaps)] [at 0] [out '()])
    (cond
      [(null? rest) (reverse out)]
      [else
       (define end (chk+ at (car rest)))
       (walk (cdr rest) (cdr open) (chk+ end (car open)) (cons (cons at end) out))])))

;; The two limits one run's reading stands between: how far back it may reach and how
;; far forward, both relative to the first base character.
;;
;; Inside the compound the limit is the adjacent base CHARACTER less §F.1's one em in
;; the ruby character size -- the space forced in between them is the compound's own
;; and the reading may stand in all of it. Outside, it is whatever §3.3.8 allowed,
;; measured from the neighbor's own edge.
(define (run-limits extents units lead trail gaps)
  (define count (length extents))
  (for/list ([here (in-list extents)] [unit (in-list units)] [index (in-naturals)])
    (define back
      (if (zero? index)
          (chk- (- (car gaps)) (lead unit))
          (chk- (cdr (list-ref extents (sub1 index))) unit)))
    (define forward
      (if (= index (sub1 count))
          (chk+ (chk+ (cdr here) (last gaps)) (trail unit))
          (chk+ (car (list-ref extents (add1 index))) unit)))
    (cons back forward)))

;; Whether every run has somewhere to stand once `gaps` has been forced in: the
;; earliest arrangement is the one that leaves the most room for the runs after it,
;; so a compound that does not fit in that one does not fit at all.
(define (compound-fits? readings limits)
  (let walk ([rest readings] [here limits] [at #f])
    (cond
      [(null? rest) #t]
      [else
       (define bound (car here))
       (define lower (if at (max (car bound) at) (car bound)))
       (define end (chk+ lower (car rest)))
       (and (<= end (cdr bound)) (walk (cdr rest) (cdr here) end))])))

;; §F.3's own two steps, in its own order: the total is first "distributed across
;; those base characters accompanied by more than two ruby characters in accordance
;; with the number of ruby characters (or the length of ruby characters when set
;; solid)", and only then does each base character "expand the preceding and
;; succeeding inter-character spacing equally by half of the assigned space".
;;
;; Two divisions and not one. Summing the two halves that meet at a boundary and
;; dividing the total over the sums instead is the same arithmetic wherever nothing
;; is left over and a different answer wherever something is: a base character
;; assigned an odd amount has one unit that halving cannot place, and it is placed by
;; `adjustment.remainder` at that base character rather than at the whole compound.
(define (compound-gaps total asking style)
  (define assigned (shares total asking style))
  (for/list ([index (in-range (add1 (length asking)))])
    (chk+ (if (> index 0) (second (shares (list-ref assigned (sub1 index)) (list 1 1) style)) 0)
          (if (< index (length asking)) (first (shares (list-ref assigned index) (list 1 1) style)) 0))))

;; §F.3's total, as the least one the compound fits at
;; (docs/decisions/ruby-distribution-and-rounding.md). The formula states the total
;; as a function of a layout the total itself produces, so what is evaluated here is
;; the fixed point rather than the sentence, and the fixed point is found by
;; bisection over a predicate that only ever turns true.
(define (least-total readings widths asking units lead trail style)
  (define (fits? total)
    (define gaps (compound-gaps total asking style))
    (define extents (run-extents widths gaps))
    (compound-fits? readings (run-limits extents units lead trail gaps)))
  (define ceiling (chk+ (total-of readings) (total-of widths)))
  (cond
    [(fits? 0) 0]
    [(not (fits? ceiling)) ceiling]
    [else
     (let bisect ([low 0] [high ceiling])
       (cond
         [(>= (add1 low) high) high]
         [else
          (define middle (div-trunc (chk+ low high) 2))
          (if (fits? middle) (bisect low middle) (bisect middle high))]))]))

(define (plan-compound one bases lead trail style)
  (define runs (ruby-runs one))
  (define units (for/list ([piece (in-list runs)]) (run-unit piece)))
  (define readings (for/list ([piece (in-list runs)]) (reading-width piece)))
  (define widths
    (for/list ([piece (in-list runs)]) (total-of (map base-advance (run-bases piece bases)))))
  ;; §F.3: "inter-character spacing can be expanded only for those base characters
  ;; which are accompanied by more than two ruby characters", and the share is "in
  ;; accordance with the number of ruby characters (or the length of ruby characters
  ;; when set solid)". Two or fewer stay with their base by §F.1 and ask for nothing,
  ;; whether or not they happen to be wider than it.
  (define asking
    (for/list ([piece (in-list runs)] [one-reading (in-list readings)])
      (if (> (length (run-annotations piece)) 2) one-reading 0)))
  (define total (least-total readings widths asking units lead trail style))
  (define gaps (compound-gaps total asking style))
  (define extents (run-extents widths gaps))
  (define limits (run-limits extents units lead trail gaps))
  ;; The furthest forward each run may stand, walked from the last backwards: a run
  ;; that stood any further forward would leave the run after it nowhere to go.
  (define latest
    (let walk ([rest (reverse (map list readings limits))] [ahead #f] [out '()])
      (cond
        [(null? rest) out]
        [else
         (define width (first (car rest)))
         (define forward (cdr (second (car rest))))
         (define here
           (if ahead (min (chk- forward width) (chk- ahead width)) (chk- forward width)))
         (walk (cdr rest) here (cons here out))])))
  ;; §F.2's first choice: a run's excess goes over the succeeding character as far as
  ;; that reaches, and over the preceding one for what is left.
  (define natural
    (for/list ([one-reading (in-list readings)] [one-width (in-list widths)]
               [here (in-list extents)] [bound (in-list limits)])
      (define over (max 0 (chk- one-reading one-width)))
      (define ahead (max 0 (chk- (cdr bound) (cdr here))))
      (chk- (car here) (max 0 (chk- over ahead)))))
  ;; Each run at its natural place, moved on where the run before it has not ended
  ;; and moved back where standing there would crowd the runs after it out.
  (define places
    (let walk ([rest (map list readings natural latest limits)] [after #f] [out '()])
      (cond
        [(null? rest) (reverse out)]
        [else
         (define width (first (car rest)))
         (define want (second (car rest)))
         (define furthest (third (car rest)))
         (define back (car (fourth (car rest))))
         (define at (min furthest (max want back (or after want))))
         (walk (cdr rest) (chk+ at width) (cons at out))])))
  (define separations (make-hash))
  (define tails (make-hash))
  (for ([piece (in-list runs)] [amount (in-list gaps)])
    (define here (run-bases piece bases))
    (when (and (pair? here) (positive? amount))
      (hash-set! separations (base-index (car here)) amount)))
  (define final (last gaps))
  (when (positive? final)
    (hash-set! tails (base-index (vector-ref bases (sub1 (vector-length bases)))) final))
  (define anchor (base-index (vector-ref bases 0)))
  (plan separations tails
        (append*
         (for/list ([piece (in-list runs)] [at (in-list places)])
           (let walk ([rest (run-annotations piece)] [at at] [out '()])
             (cond
               [(null? rest) (reverse out)]
               [else
                (define found (car rest))
                (walk (cdr rest)
                      (chk+ at (annotation-advance found))
                      (cons (attachment (ruby-index one) anchor 0 0 at
                                        (annotation-start found) (annotation-end found)
                                        (annotation-advance found) (annotation-size found) #f)
                            out))]))))))
