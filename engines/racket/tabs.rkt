#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; §3.6: tab setting.
;;
;; A tab sign is a character of the caller's own text -- U+0009, which Appendix A
;; lists under no class at all -- and what it does is take up whatever space is
;; needed to put what follows it at a stop. So its advance is not a property of the
;; sign: it is a property of where the sign happens to stand on the line, which is
;; not known until the line is being measured.
;;
;; What §3.6.2 states is four kinds of stop, and they differ only in WHAT is put at
;; the stop:
;;
;;   start      the string after the sign begins there
;;   end        it ends there
;;   center     its middle is there
;;   character  the first occurrence of a stated character is there
;;
;; The string is what stands between this sign and the next one, or the line end.
;;
;; Which stop, and what happens when there is none
;;
;; §3.6.3 says the signs of a line correspond with the stops of that line "in
;; order", and the only order a line has is position: each sign takes the nearest
;; stop AHEAD of the cursor, whatever order the request lists them in. A sign whose
;; stops the line has all gone past has nothing to reach, and §3.6.3's fourth case
;; says the string "should be set from the tab position of the next line" -- which
;; says where the string goes and not what happens to the line. This engine ends
;; the line before the sign, and does so at boundaries Table 2 would never allow one
;; to end at, because the cut is §3.6's and not §3.1's: it is a line boundary rather
;; than a break opportunity, so the only thing that can withhold it is there being
;; no boundary at that point at all.
;;
;; The one place the fourth case has nothing to say is a sign at the line head,
;; where there is no earlier boundary to send it back to. Such a sign keeps its line
;; and takes one em of the paragraph's own size, which is a number §3.6 never
;; mentions and which this engine states here rather than deriving.
;;
;; Validation
;;
;; Two things are refused rather than answered. A stop at or past the measure is not
;; a position in the line: §3.6 says a stop is a position in the line and says
;; nothing about the ends of one, so a stop that is not strictly inside is an input
;; error, checked whether or not the source holds a sign at all. And §3.6.1's "it is
;; necessary to set the same numbers of tab positions ... as the number of tab
;; signs" counts signs in a LINE, which composition decides; the only division into
;; lines validation can see is the caller's own mandatory breaks, so a stretch
;; between two of them holding more signs than there are stops is refused, and one
;; holding fewer is answered.

(require racket/list
         "arith.rkt"
         "model.rkt"
         "spacing.rkt")

(provide tab-sign?
         validate-tabs
         stops-in-order
         tab-target)

;; The one character §3.6 is about.
(define tab-character #\tab)

(define (tab-sign? para one)
  (define text (source-slice (paragraph-source para) (item-start one) (item-end one)))
  (and (= (string-length text) 1) (char=? (string-ref text 0) tab-character)))

;; The caller's stops, in the order they stand along the line.
(define (stops-in-order para)
  (sort (paragraph-tab-stops para) < #:key tab-stop-position))

;; §3.6 and §3.6.1's two refusals.
(define (validate-tabs para items)
  (define measure (paragraph-line-extent para))
  (for ([one (in-list (paragraph-tab-stops para))])
    (unless (< (tab-stop-position one) measure)
      (fail-input "input.tab-stop-outside-line: a tab stop at ~a is not inside a line of ~a"
                  (tab-stop-position one)
                  measure)))
  (define stops (length (paragraph-tab-stops para)))
  (define mandatory
    (for/list ([one (in-list (paragraph-breaks para))] #:when (eq? (brk-kind one) 'mandatory))
      (brk-offset one)))
  (let count ([index 0] [signs 0])
    (cond
      [(>= index (vector-length items))
       (unless (<= signs stops)
         (fail-input "input.tab-count: a stretch of the paragraph holds ~a tab sign(s) and the request states ~a tab stop(s)"
                     signs stops))]
      [else
       (define one (vector-ref items index))
       (define cut? (and (memv (item-start one) mandatory) #t))
       (when cut?
         (unless (<= signs stops)
           (fail-input "input.tab-count: a stretch of the paragraph holds ~a tab sign(s) and the request states ~a tab stop(s)"
                       signs stops)))
       (count (add1 index)
              (+ (if cut? 0 signs) (if (tab-sign? para one) 1 0)))])))

;; Where the string after a sign has to start, for the sign to put it at `stop`.
;;
;; `widths` is the string's own characters in order, as `(width . text)` pairs, and
;; `stop` is the tab stop. The answer is a position on the line, which may be behind
;; the cursor: a stop the string is already past is one the sign cannot reach.
(define (tab-target stop widths)
  (define position (tab-stop-position stop))
  (define whole (for/fold ([sum 0]) ([one (in-list widths)]) (chk+ sum (car one))))
  (case (tab-stop-alignment stop)
    [(start) position]
    [(end) (chk- position whole)]
    [(center) (chk- position (div-trunc whole 2))]
    [(character)
     (define wanted (tab-stop-character stop))
     (define offset
       (let walk ([rest widths] [at 0])
         (cond
           [(null? rest) #f]
           [(let ([text (cdr (car rest))])
              (for/or ([character (in-string text)]) (char=? character wanted)))
            at]
           [else (walk (cdr rest) (chk+ at (car (car rest))))])))
     ;; A stop that names a character the string does not hold puts the string's own
     ;; start there, which is what `start` does and is the only position left.
     (chk- position (or offset 0))]
    [else position]))
