#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; Whole paragraphs, composed.
;;
;; The modules below `compose.rkt` are tested at their own coordinates; this file
;; tests the answer, because most of what can go wrong in a layout engine is a
;; number that is right in one table and wrong once it has been through five. Each
;; case here is small enough to compute by hand from the sections it names, and the
;; comment says which sections those are.
;;
;; The geometry the protocol carries has two halves and they are different numbers
;; whenever a line was adjusted: a placement's `advance` is the advance the line was
;; *composed from* -- the character's own plus what Table 1 states after it -- and
;; its `inline` is where the character actually stands once §3.8.3's or §3.8.4's
;; ladder has run. Several cases below check both at once for that reason.

(require rackunit racket/list "../model.rkt" "../compose.rkt")

;; One cluster of a request.
(define (cluster* start end advance #:frame [frame #f] #:size [size #f] #:role [role #f])
  (define base (hasheq 'range (list start end) 'advance advance))
  (let* ([one (if frame (hash-set base 'frame frame) base)]
         [two (if size (hash-set one 'size (hasheq 'inline size 'block size)) one)])
    (if role (hash-set two 'role role) two)))

(define (request source clusters extent
                 #:breaks [breaks '()]
                 #:style [style "jlreq-2020"]
                 #:alignment [alignment #f]
                 #:writing-mode [writing-mode #f]
                 #:indent [indent #f]
                 #:constructs [constructs '()])
  (define base
    (hasheq 'source source
            'size (hasheq 'inline 1000 'block 1000)
            'frame "full-em"
            'clusters clusters
            'line_extent extent
            'style style))
  (let* ([a (if (null? breaks) base (hash-set base 'breaks breaks))]
         [b (if (null? constructs) a (hash-set a 'constructs constructs))]
         [c (if alignment (hash-set b 'alignment alignment) b)]
         [d (if writing-mode (hash-set c 'writing_mode writing-mode) c)])
    (if indent (hash-set d 'first_line_indent indent) d)))

(define (break* offset kind)
  (hasheq 'offset offset 'kind kind))

;; A layout as `(list (list range inline-origin inline-extent (list (index inline advance) ...)) ...)`.
(define (layout-of body)
  (define-values (lines notes) (compose (parse-request body)))
  (for/list ([one (in-list lines)])
    (list (list (line-start one) (line-end one))
          (line-inline-origin one)
          (line-inline-extent one)
          (for/list ([each (in-list (line-clusters one))])
            (list (placed-index each) (placed-inline each) (placed-advance each))))))

(define (notes-of body)
  (define-values (lines notes) (compose (parse-request body)))
  (for/list ([one (in-list notes)]) (list (first one) (third one) (fourth one))))

(define (blocks-of body)
  (define-values (lines notes) (compose (parse-request body)))
  (for/list ([one (in-list lines)])
    (list (line-block-origin one)
          (line-block-extent one)
          (for/list ([each (in-list (line-clusters one))]) (placed-block each)))))

(module+ test
  ;; ------------------------------------------------------------------
  ;; §3.8.1: lines, and the geometry of one
  ;; ------------------------------------------------------------------

  ;; Two ideographs, a measure that holds one, one stated opportunity.
  (check-equal? (layout-of (request "日本"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 1000))
                                    1000
                                    #:breaks (list (break* 3 "allowed"))))
                '(((0 3) 0 1000 ((0 0 1000))) ((3 6) 0 1000 ((1 0 1000)))))

  ;; A boundary the caller did not state is not an opportunity: §3.8.1 composes at
  ;; the places the caller says line breaking is not prohibited at.
  (check-equal? (layout-of (request "日本" (list (cluster* 0 3 1000) (cluster* 3 6 1000)) 1000))
                '(((0 6) 0 2000 ((0 0 1000) (1 1000 1000)))))
  (check-equal? (notes-of (request "日本" (list (cluster* 0 3 1000) (cluster* 3 6 1000)) 1000))
                '(("layout.overfull" 0 6)))

  ;; ------------------------------------------------------------------
  ;; Appendix B: the amount, and whose em it is
  ;; ------------------------------------------------------------------

  ;; (cl-19, cl-27) is `1/4 be` -- a quarter of the ideograph's own em, not of the
  ;; Western character's, which is what §B.1's two words for a half em are for.
  (check-equal? (layout-of (request "日A"
                                    (list (cluster* 0 3 1000)
                                          (cluster* 3 4 400 #:frame "proportional" #:size 400))
                                    3000))
                '(((0 4) 0 1650 ((0 0 1250) (1 1250 400)))))

  ;; §B.2 note 3: two middle dots take the sum of their own two quarter ems, and
  ;; §3.1.9's quarter em stands after the last one at the line end.
  (check-equal? (layout-of (request "日・"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 500 #:frame "half-em"))
                                    4000))
                '(((0 6) 0 2000 ((0 0 1250) (1 1250 750)))))

  ;; §3.1.3, in the union of the two locales: a middle dot inside a unit symbol
  ;; carries no space of its own, and the neighbor's own stays.
  (check-equal? (layout-of (request "日・日"
                                    (list (cluster* 0 3 1000)
                                          (cluster* 3 6 1000 #:role "unit-symbol")
                                          (cluster* 6 9 1000))
                                    9000))
                '(((0 9) 0 3000 ((0 0 1000) (1 1000 1000) (2 2000 1000)))))

  ;; §3.1.6's first case: a sentence-final dividing punctuation mark takes one em
  ;; after it, and the line end withdraws it.
  (check-equal? (layout-of (request "日？日"
                                    (list (cluster* 0 3 1000)
                                          (cluster* 3 6 1000 #:role "sentence-terminator")
                                          (cluster* 6 9 1000))
                                    4000
                                    #:alignment "start"))
                '(((0 9) 0 4000 ((0 0 1000) (1 1000 2000) (2 3000 1000)))))
  (check-equal? (layout-of (request "日？日"
                                    (list (cluster* 0 3 1000)
                                          (cluster* 3 6 1000 #:role "sentence-terminator")
                                          (cluster* 6 9 1000))
                                    3000
                                    #:alignment "start"
                                    #:breaks (list (break* 6 "allowed"))))
                '(((0 6) 0 2000 ((0 0 1000) (1 1000 1000))) ((6 9) 0 1000 ((2 0 1000)))))

  ;; §3.1.6's third Note, under `quarter-em`: the mark's own quarter em on both
  ;; sides, at a coordinate Table 1 states nothing at.
  (check-equal? (layout-of (request "日？日"
                                    (list (cluster* 0 3 1000)
                                          (cluster* 3 6 1000 #:role "sentence-medial")
                                          (cluster* 6 9 1000))
                                    4000
                                    #:style (hasheq 'profile "jlreq-2020"
                                                    'spacing.sentence_medial_dividing_mark "quarter-em")))
                '(((0 9) 0 3500 ((0 0 1250) (1 1250 1250) (2 2500 1000)))))

  ;; §3.1.5 pattern 2: an opening bracket stands half an em in from the head of
  ;; every line, over and above the paragraph's own indent.
  (check-equal? (layout-of (request "「日「日"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 1000)
                                          (cluster* 6 9 1000) (cluster* 9 12 1000))
                                    5000
                                    #:indent 1000
                                    #:breaks (list (break* 6 "mandatory"))
                                    #:style (hasheq 'profile "jlreq-2020"
                                                    'spacing.line_head_opening_bracket "pattern-2")))
                '(((0 6) 0 3500 ((0 1500 1000) (1 2500 1000)))
                  ((6 12) 0 2500 ((2 500 1000) (3 1500 1000)))))

  ;; ------------------------------------------------------------------
  ;; Appendix D: the ladder, and the two geometries
  ;; ------------------------------------------------------------------

  ;; §B.2 note 2 and Table 3's `1/2=0`: the half em after a closing bracket at the
  ;; line end is a half em or it is nothing. The advance reports the half em the
  ;; line was composed from either way.
  (check-equal? (layout-of (request "亜。"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 500 #:frame "half-em"))
                                    2000))
                '(((0 6) 0 2000 ((0 0 1000) (1 1000 1000)))))
  (check-equal? (layout-of (request "亜。"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 500 #:frame "half-em"))
                                    1900))
                '(((0 6) 0 1500 ((0 0 1000) (1 1000 1000)))))
  ;; ... and Table 4 states the same half em rigid, so the line overruns instead.
  (check-equal? (layout-of (request "亜。"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 500 #:frame "half-em"))
                                    1700
                                    #:style (hasheq 'profile "jlreq-2020"
                                                    'adjustment.reduction_table "table-4")))
                '(((0 6) 0 2000 ((0 0 1000) (1 1000 1000)))))

  ;; §D.2 note 3: a comma before a middle dot is the comma's own half em and the
  ;; dot's own quarter, and the two are reduced at two different priorities -- the
  ;; dot's at the fourth stage and the comma's at the fifth, which is why a line
  ;; that needs a quarter em back takes it from the dot alone.
  (check-equal? (layout-of (request "、・末"
                                    (list (cluster* 0 3 500 #:frame "half-em")
                                          (cluster* 3 6 500 #:frame "half-em")
                                          (cluster* 6 9 1000))
                                    1250
                                    #:alignment "start"
                                    #:breaks (list (break* 6 "mandatory"))))
                '(((0 6) 0 1250 ((0 0 1250) (1 750 750))) ((6 9) 0 1000 ((2 0 1000)))))

  ;; §3.8.3 (e): the quarter em between Japanese and Latin text goes to an eighth
  ;; and no further, and the advance still reports the quarter.
  (check-equal? (layout-of (request "1A末"
                                    (list (cluster* 0 1 500 #:frame "half-em")
                                          (cluster* 1 2 500 #:frame "proportional" #:size 500)
                                          (cluster* 2 5 1000))
                                    1750
                                    #:breaks (list (break* 1 "allowed"))
                                    #:style (hasheq 'profile "jlreq-2020"
                                                    'classification.grouped_numeral_qualification "by-role")))
                '(((0 5) 0 2125 ((0 0 500) (1 500 750) (2 1125 1000)))))

  ;; §3.8.2's hanging punctuation closes what the ladder could not, and no more.
  (check-equal? (layout-of (request "亜。"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 500 #:frame "half-em"))
                                    1300
                                    #:style (hasheq 'profile "jlreq-2020"
                                                    'adjustment.hanging_punctuation "hanging")))
                '(((0 6) 0 1300 ((0 0 1000) (1 1000 1000)))))

  ;; ------------------------------------------------------------------
  ;; Appendix E: the expansion ladder
  ;; ------------------------------------------------------------------

  ;; Table 6's `0-1/4 stage 3` at (cl-19, cl-19), on a justified line that is not
  ;; the last one.
  (check-equal? (layout-of (request "日日末"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 1000) (cluster* 6 9 1000))
                                    2500
                                    #:alignment "justify"
                                    #:breaks (list (break* 6 "mandatory"))))
                '(((0 6) 0 2500 ((0 0 1000) (1 1500 1000))) ((6 9) 0 1000 ((2 0 1000)))))

  ;; §E.2 note 4: two inseparable characters of DIFFERENT kinds open a quarter em,
  ;; and two of the same kind do not open at all -- nor do they break (§C.2 note 5).
  (check-equal? (layout-of (request "—…末"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 1000) (cluster* 6 9 1000))
                                    2250
                                    #:alignment "justify"
                                    #:breaks (list (break* 3 "allowed") (break* 6 "mandatory"))))
                '(((0 6) 0 2250 ((0 0 1000) (1 1250 1000))) ((6 9) 0 1000 ((2 0 1000)))))
  (check-equal? (layout-of (request "——末"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 1000) (cluster* 6 9 1000))
                                    1000
                                    #:alignment "start"
                                    #:breaks (list (break* 3 "allowed"))))
                '(((0 9) 0 3000 ((0 0 1000) (1 1000 1000) (2 2000 1000)))))

  ;; §3.8.4's own Note: `rigid` takes the Japanese-Latin quarter em out of every
  ;; step, step (d) included, so a justified line comes up short instead.
  (define latin
    (list (cluster* 0 3 1000)
          (cluster* 3 4 1000 #:frame "proportional")
          (cluster* 4 7 1000)
          (cluster* 7 10 1000)))
  (check-equal? (layout-of (request "日A末末" latin 3750
                                    #:alignment "justify"
                                    #:breaks (list (break* 7 "mandatory"))
                                    #:style (hasheq 'profile "jlreq-2020"
                                                    'adjustment.japanese_latin_expansion_ceiling "rigid")))
                '(((0 7) 0 3500 ((0 0 1250) (1 1250 1250) (2 2500 1000))) ((7 10) 0 1000 ((3 0 1000)))))

  ;; ------------------------------------------------------------------
  ;; §3.5.3: where a short line stands
  ;; ------------------------------------------------------------------

  (check-equal? (layout-of (request "日本" (list (cluster* 0 3 1000) (cluster* 3 6 1000)) 5000
                                    #:alignment "end"))
                '(((0 6) 3000 2000 ((0 3000 1000) (1 4000 1000)))))
  (check-equal? (layout-of (request "日本" (list (cluster* 0 3 1000) (cluster* 3 6 1000)) 5000
                                    #:alignment "center"))
                '(((0 6) 1500 2000 ((0 1500 1000) (1 2500 1000)))))
  (check-equal? (layout-of (request "日本" (list (cluster* 0 3 1000) (cluster* 3 6 1000)) 5000
                                    #:alignment "start"))
                '(((0 6) 0 2000 ((0 0 1000) (1 1000 1000)))))

  ;; ------------------------------------------------------------------
  ;; §3.2: the other writing mode
  ;; ------------------------------------------------------------------

  ;; Lines progress leftward, so the block coordinate runs the other way.
  (check-equal? (blocks-of (request "日本"
                                    (list (cluster* 0 3 1000) (cluster* 3 6 1000))
                                    1000
                                    #:writing-mode "vertical-rl"
                                    #:breaks (list (break* 3 "allowed"))))
                '((0 1000 (0)) (-1000 1000 (-1000))))

  ;; §3.2.5: a run is one thing on the line, set across it and centered on it. What
  ;; it takes ALONG the line is the tallest member's block size, and what it takes
  ;; ACROSS is the string's own width or the paragraph's own size, whichever is
  ;; larger.
  (check-equal? (layout-of (request "日12日"
                                    (list (cluster* 0 3 1000)
                                          (cluster* 3 4 300 #:frame "proportional")
                                          (cluster* 4 5 433 #:frame "proportional")
                                          (cluster* 5 8 1000))
                                    16000
                                    #:writing-mode "vertical-rl"
                                    #:constructs (list (hasheq 'kind "tate-chu-yoko" 'range '(3 5)))))
                '(((0 8) 0 3000 ((0 0 1000) (1 1000 300) (2 1000 433) (3 2000 1000)))))
  (check-equal? (blocks-of (request "日12日"
                                    (list (cluster* 0 3 1000)
                                          (cluster* 3 4 300 #:frame "proportional")
                                          (cluster* 4 5 433 #:frame "proportional")
                                          (cluster* 5 8 1000))
                                    16000
                                    #:writing-mode "vertical-rl"
                                    #:constructs (list (hasheq 'kind "tate-chu-yoko" 'range '(3 5)))))
                '((0 1000 (0 -366 -66 0))))

  ;; ------------------------------------------------------------------
  ;; Appendix C: what may be broken
  ;; ------------------------------------------------------------------

  ;; §C.2 note 11: a Western character used as a quantity symbol holds on to the
  ;; postfixed abbreviation after it, so the caller's own opportunity is refused.
  (check-equal? (layout-of (request "A％"
                                    (list (cluster* 0 1 1000 #:frame "proportional" #:role "quantity-symbol")
                                          (cluster* 1 4 1000))
                                    1000
                                    #:alignment "start"
                                    #:breaks (list (break* 1 "allowed"))))
                '(((0 4) 0 2000 ((0 0 1000) (1 1000 1000)))))

  ;; §C.3: the same boundary at the four levels. A prolonged sound mark may not
  ;; begin a line at `very-strict`, and may at every other level.
  (define sound-mark
    (lambda (level)
      (length (layout-of (request "日ー"
                                  (list (cluster* 0 3 1000) (cluster* 3 6 1000))
                                  1000
                                  #:alignment "start"
                                  #:breaks (list (break* 3 "allowed"))
                                  #:style (hasheq 'profile "jlreq-2020" 'kinsoku.level level))))))
  (check-equal? (sound-mark "very-loose") 2)
  (check-equal? (sound-mark "loose") 2)
  (check-equal? (sound-mark "strict") 2)
  (check-equal? (length (layout-of (request "日ー"
                                            (list (cluster* 0 3 1000) (cluster* 3 6 1000))
                                            1000
                                            #:alignment "start"
                                            #:breaks (list (break* 3 "allowed"))
                                            #:style (hasheq 'profile "jlreq-2020"
                                                            'kinsoku.level "very-strict"
                                                            'kinsoku.grouped_numeral_before_western "unbreakable"
                                                            'kinsoku.relaxation_mechanism "matrix"))))
                1)

  ;; §B.2 note 15 under `reclassify`: the mark becomes katakana, and a prefixed
  ;; abbreviation still holds on to whatever follows it -- (cl-12, cl-16) is `not`
  ;; for a reason that has nothing to do with the sound mark.
  (check-equal? (length (layout-of (request "№ー"
                                            (list (cluster* 0 3 1000) (cluster* 3 6 1000))
                                            1000
                                            #:alignment "start"
                                            #:breaks (list (break* 3 "allowed")))))
                1)
  ;; ... and under `matrix` the boundary is opened instead of the class changed.
  (check-equal? (length (layout-of (request "№ー"
                                            (list (cluster* 0 3 1000) (cluster* 3 6 1000))
                                            1000
                                            #:alignment "start"
                                            #:breaks (list (break* 3 "allowed"))
                                            #:style (hasheq 'profile "jlreq-2020"
                                                            'kinsoku.relaxation_mechanism "matrix"))))
                2)

  ;; ------------------------------------------------------------------
  ;; §3.2.2 and §B.2 note 13: the Western word space at a line edge
  ;; ------------------------------------------------------------------

  (check-equal? (layout-of (request " A B"
                                    (list (cluster* 0 1 333 #:frame "proportional")
                                          (cluster* 1 2 500 #:frame "proportional")
                                          (cluster* 2 3 333 #:frame "proportional")
                                          (cluster* 3 4 500 #:frame "proportional"))
                                    2000
                                    #:alignment "justify"
                                    #:breaks (list (break* 3 "mandatory"))))
                '(((0 3) 0 500 ((0 0 0) (1 0 500) (2 500 0))) ((3 4) 0 500 ((3 0 500)))))

  ;; A space the caller set on the full em is not the space §3.2.2 measured, and
  ;; keeps the advance it was shaped with.
  (check-equal? (layout-of (request "日 "
                                    (list (cluster* 0 3 1000) (cluster* 3 4 1000))
                                    4000))
                '(((0 4) 0 2000 ((0 0 1000) (1 1000 1000)))))

  ;; ------------------------------------------------------------------
  ;; A request with nothing in it
  ;; ------------------------------------------------------------------

  (check-equal? (layout-of (request "" '() 1000)) '())
  (check-equal? (notes-of (request "" '() 1000)) '()))
