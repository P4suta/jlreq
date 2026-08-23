#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; §3.3.9 and §3.7.1: the ornamented character complex (cl-21).
;;
;; Two constructs land in one class and are set two different ways, and the
;; difference is how many complexes each one is.
;;
;; **Emphasis dots (§3.3.9).** A mark half the size of its base character, centered
;; on it, one per base character. It takes no advance: the line is exactly as wide
;; with the dots as without them. JLReq never says how many ornamented character
;; complexes a run of emphasized characters is, and §B.2 note 9, §C.2 note 6 and
;; §E.2 note 5 are all stated about "two consecutive characters belonging to the
;; same ornamented character complex" -- so the number matters. This engine answers
;; ONE PER CHARACTER: Table 6's quarter em opens between two emphasized characters
;; of one run, and a line break stated between them is answered.
;;
;; **Superscripts and reference marks (§3.7.1).** The whole construct is one
;; complex. §3.7.1 says its geometry is "implementation definable" and that the
;; annotation is "set after the base character"; this engine sets it ACROSS the
;; complex instead -- centered on it, hanging over both neighbors where it is
;; longer, and opening the line nowhere. A line break stated inside one is refused,
;; and Table 6 opens nothing between two of its characters.
;;
;; What "centered on the base characters" is centered on
;;
;; The center of the advance the LINE gave the base, spacing and all -- not the
;; center of its own em box. The two readings part wherever Table 1 states a space
;; after the base: an emphasis run is cl-21, so a quarter em stands after it before
;; an ideograph, and the mark sits an eighth of an em later than the em-box reading
;; would put it. That is why an attachment from here carries a SPAN rather than an
;; offset: the offset is not known until the line is measured.

(require racket/list
         "arith.rkt"
         "model.rkt"
         "spacing.rkt")

(provide (struct-out ornament)
         ornament-constructs
         plan-ornament)

;; One §3.3.9 or §3.7.1 construct.
;;
;; `mark` is the character §3.3.9 repeats, or #f; `annotation` is the shaped text
;; §3.7.1 sets, as a list of `(start end advance size)`, or '().
(struct ornament (index kind start end mark annotation) #:transparent)

(define (field what object name)
  (define found (hash-ref object name #f))
  (unless found
    (fail-input "~a does not state ~a" what name))
  found)

;; The shaped text of a superscript or a reference mark.
(define (read-annotation value)
  (define what "an annotation")
  (unless (hash? value)
    (fail-input "~a is not a JSON object" what))
  (define size (field what value 'size))
  (define default (extent (field what size 'inline) (field what size 'block)))
  (for/list ([one (in-list (field what value 'clusters))])
    (define range (field what one 'range))
    (define own (hash-ref one 'size #f))
    (list (car range)
          (cadr range)
          (field what one 'advance)
          (if own (extent (field what own 'inline) (field what own 'block)) default))))

(define (ornament-constructs para)
  (for/list ([one (in-list (paragraph-constructs para))]
             [index (in-naturals)]
             #:when (memq (construct-kind one) '(emphasis-dots script reference-mark)))
    (define payload (construct-payload one))
    (ornament index
              (construct-kind one)
              (construct-start one)
              (construct-end one)
              (let ([found (assq 'mark payload)]) (and found (cdr found)))
              (let ([found (assq 'annotation payload)])
                (if found (read-annotation (cdr found)) '())))))

;; The attachments one construct puts on the line.
;;
;; `bases` is a vector of `(index start advance size)` for the construct's own base
;; items, in line order.
(define (plan-ornament one bases)
  (case (ornament-kind one)
    [(emphasis-dots)
     ;; §3.3.3's own ratio, which §3.3.9 inherits: the mark is half the size of the
     ;; base character it marks, and each base character has its own.
     (for/list ([found (in-vector bases)])
       (define size (fourth found))
       (define half (extent (div-trunc (extent-inline size) 2) (div-trunc (extent-block size) 2)))
       (attachment (ornament-index one) (first found) 1 (extent-inline half) 0 0 0 0 half
                   (ornament-mark one)))]
    [else
     (define anchor (first (vector-ref bases 0)))
     (define span (vector-length bases))
     (define whole
       (for/fold ([sum 0]) ([found (in-list (ornament-annotation one))]) (chk+ sum (third found))))
     (let walk ([rest (ornament-annotation one)] [at 0] [out '()])
       (cond
         [(null? rest) (reverse out)]
         [else
          (define found (car rest))
          (walk (cdr rest)
                (chk+ at (third found))
                (cons (attachment (ornament-index one) anchor span whole at
                                  (first found) (second found) (third found) (fourth found) #f)
                      out))]))]))
