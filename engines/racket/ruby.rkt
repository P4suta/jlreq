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
;; `separations` maps a boundary -- the index of the item the space stands BEFORE --
;; to the amount forced in there. `attachments` is where the reading goes.
(struct plan (separations attachments) #:transparent)

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

;; The classes whose Table 1 cell says a reading may extend over the character
;; itself and that are not kana. The kana are answered by script instead: §3.3.8
;; rule 2 names "hiragana (cl-15), katakana (cl-16), prolonged sound mark (cl-10) or
;; small kana (cl-11)", which is two scripts and two classes spelled in them, and
;; the two readings part at exactly the marks they disagree about -- U+30FC is cl-10
;; and Script=Common, and the katakana iteration marks are cl-09 and Script=Katakana.
(define (structural-hang? class)
  (and (memv class '(1 8 14)) #t))

;; Whether a reading may extend over `neighbor` itself, and how far.
;;
;; `neighbor-script` is what `spec/derived/scripts.tsv` names the character, as a
;; string, or #f where the item is not one code point.
;;
;; Every permission is "up to the size of the ruby character", which is the reading's
;; own em and not the neighbor's.
(define (character-hang neighbor-class neighbor-script style em)
  (define chosen (answer style "ruby.overhang_kana"))
  (cond
    [(string=? chosen "any") em]
    [(structural-hang? neighbor-class) em]
    [(string=? chosen "none") 0]
    [(equal? neighbor-script "Hiragana") em]
    [(and (equal? neighbor-script "Katakana") (not (string=? chosen "jis"))) em]
    [else 0]))

;; ----------------------------------------------------------------------------
;; Planning one construct
;; ----------------------------------------------------------------------------

;; `bases` is a vector of `(index start advance)` triples, one per base item of the
;; construct, in line order -- the item's own place in the paragraph, the byte offset
;; its text starts at, and the advance it was shaped with. `lead` and `trail` are how
;; far the reading may hang before the first base character and after the last.
(define (plan-ruby one bases lead trail style)
  (define em (extent-inline (ruby-em one)))
  (define compound?
    (and (eq? (ruby-kind one) 'jukugo)
         (for/or ([piece (in-list (ruby-runs one))]) (>= (length (run-annotations piece)) 3))))
  (cond
    [compound? (plan-compound one bases lead trail style)]
    [(and (eq? (ruby-kind one) 'group) (> (vector-length bases) 1))
     (plan-distributed one bases style)]
    [else (plan-per-run one bases lead trail style)]))

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
      (define offset
        (cond
          [(positive? overflow) (if katatsuki? 0 (- (div-trunc overflow 2)))]
          [katatsuki? 0]
          [else (div-trunc (chk- width reading) 2)]))
      (define split (shares overflow (list 1 1) style))
      (define first? (zero? rank))
      (define last? (= rank (sub1 (length (ruby-runs one)))))
      ;; A run's own excess may hang over what stands outside the construct; inside
      ;; it, the next base character is where the neighboring run's own reading is,
      ;; so the excess is forced rather than hung.
      (define before (max 0 (chk- (first split) (if first? lead 0))))
      (define after (max 0 (chk- (second split) (if last? trail 0))))
      (when (positive? before)
        (hash-update! separations anchor (lambda (found) (max found before)) 0))
      (when (positive? after)
        (define beyond (add1 (base-index (last here))))
        (hash-update! separations beyond (lambda (found) (max found after)) 0))
      (let walk ([rest (run-annotations piece)] [at offset])
        (unless (null? rest)
          (define found (car rest))
          (set! attachments
                (cons (attachment (ruby-index one) anchor 0 0 at
                                  (annotation-start found) (annotation-end found)
                                  (annotation-advance found) (annotation-size found) #f)
                      attachments))
          (walk (cdr rest) (chk+ at (annotation-advance found)))))))
  (plan separations (reverse attachments)))

;; ----------------------------------------------------------------------------
;; §3.3.6: a group run over two or more base characters
;; ----------------------------------------------------------------------------

(define (plan-distributed one bases style)
  (define flush? (answer-is? style "ruby.group_distribution" "flush"))
  (define piece (car (ruby-runs one)))
  (define readings (run-annotations piece))
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
     (plan (make-hash)
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
     (for ([found (in-vector bases)] [amount (in-list gaps)])
       (when (positive? amount)
         (hash-set! separations (base-index found) amount)))
     (define final (last gaps))
     (when (positive? final)
       (hash-set! separations (add1 (base-index (vector-ref bases (sub1 (vector-length bases))))) final))
     (plan separations
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

(define (plan-compound one bases lead trail style)
  (define runs (ruby-runs one))
  (define readings (for/list ([piece (in-list runs)]) (reading-width piece)))
  (define widths
    (for/list ([piece (in-list runs)]) (total-of (map base-advance (run-bases piece bases)))))
  (define reading (total-of readings))
  (define width (total-of widths))
  (define excess (max 0 (chk- reading width)))
  ;; What the two ends absorb, and what is left for the compound to open up by.
  (define used (min excess (chk+ lead trail)))
  (define lead-used (min lead used))
  (define total (chk- excess used))
  ;; §F.4: only a run whose own reading is longer than its own base asks for space,
  ;; and the share each boundary takes is proportional to the reading length of the
  ;; runs it stands between.
  (define asking
    (for/list ([one-reading (in-list readings)] [one-width (in-list widths)])
      (if (> one-reading one-width) one-reading 0)))
  (define count (length runs))
  (define weights
    (for/list ([index (in-range (add1 count))])
      (chk+ (if (> index 0) (list-ref asking (sub1 index)) 0)
            (if (< index count) (list-ref asking index) 0))))
  (define gaps (shares total weights style))
  (define separations (make-hash))
  (for ([piece (in-list runs)] [amount (in-list gaps)])
    (define here (run-bases piece bases))
    (when (and (pair? here) (positive? amount))
      (hash-set! separations (base-index (car here)) amount)))
  (define final (last gaps))
  (when (positive? final)
    (hash-set! separations (add1 (base-index (vector-ref bases (sub1 (vector-length bases))))) final))
  (define anchor (base-index (vector-ref bases 0)))
  (plan separations
        (let walk ([rest runs] [at (- (chk+ (car gaps) lead-used))] [out '()])
          (cond
            [(null? rest) (reverse out)]
            [else
             (define here (run-annotations (car rest)))
             (define-values (next placed)
               (for/fold ([at at] [placed '()]) ([found (in-list here)])
                 (values (chk+ at (annotation-advance found))
                         (cons (attachment (ruby-index one) anchor 0 0 at
                                           (annotation-start found) (annotation-end found)
                                           (annotation-advance found) (annotation-size found) #f)
                               placed))))
             (walk (cdr rest) next (append placed out))]))))
