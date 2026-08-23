#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The request, as this engine wants to read it.
;;
;; `protocol.rkt` hands over the `request` object with nothing but "it is a JSON
;; object" established. Everything below turns that object into records, and
;; refuses anything protocol.schema.json refuses: an unknown field, a value outside
;; a closed vocabulary, a number outside the i32 range, a byte range that is not a
;; range of the source.
;;
;; Why the reader is closed
;;
;; A field this engine does not understand may be the one carrying the meaning of
;; the request. Answering it as though the field were absent produces a plausible
;; wrong answer, which a structural comparison reports as a conformance difference
;; rather than as the input error it is. So an unknown key is `exit 2`, exactly as
;; it is in the envelope.
;;
;; Byte offsets, not characters
;;
;; Every `range` the protocol carries is a pair of UTF-8 byte offsets into
;; `source`, so the source is kept as bytes and a cluster's text is decoded out of
;; it on demand. Racket strings are sequences of code points and indexing one by a
;; byte offset would be wrong on every character outside ASCII, which is most of
;; this specification's subject.

(require json racket/list "arith.rkt")

(provide (struct-out extent)
         (struct-out cluster)
         (struct-out brk)
         (struct-out construct)
         (struct-out tab-stop)
         (struct-out paragraph)
         parse-request
         cluster-text
         cluster-size-of
         cluster-frame-of
         source-slice
         fail-input)

(define (fail-input template . arguments)
  (raise (exn:fail (apply format template arguments) (current-continuation-marks))))

;; An inline/block pair: a character size, in the caller's unit.
(struct extent (inline block) #:transparent)

;; One shaped cluster. `size` and `frame` are #f where the cluster states none and
;; the paragraph's own defaults stand.
(struct cluster (start end advance size frame role) #:transparent)

;; One break opportunity the caller states.
(struct brk (offset kind) #:transparent)

;; One construct. `payload` carries the kind-specific fields as an association
;; list; the milestones that implement a kind are what read them.
(struct construct (kind start end payload) #:transparent)

(struct tab-stop (position alignment character) #:transparent)

;; A whole request.
(struct paragraph
        (source              ; bytes
         size                ; extent
         frame               ; 'full-em | 'proportional | 'half-em
         clusters            ; (vectorof cluster)
         line-extent         ; positive integer
         breaks              ; (listof brk)
         constructs          ; (listof construct)
         tab-stops           ; (listof tab-stop)
         first-line-indent   ; integer
         alignment           ; 'start | 'center | 'end | 'justify | #f when unstated
         widow-minimum       ; positive integer or #f
         writing-mode        ; 'horizontal-tb | 'vertical-rl
         style)              ; the `style` value, unresolved
        #:transparent)

;; ----------------------------------------------------------------------------
;; Primitives
;; ----------------------------------------------------------------------------

(define absent (string->uninterned-symbol "absent"))

(define (field object name)
  (hash-ref object name absent))

(define (present? value)
  (not (eq? value absent)))

;; Refuse every key of `object` that is not in `known`.
(define (closed! what object known)
  (for ([name (in-list (hash-keys object))])
    (unless (memq name known)
      (fail-input "~a states `~a`, which protocol v1 does not carry" what name))))

(define (want-object what value)
  (unless (hash? value)
    (fail-input "~a is not a JSON object" what))
  value)

(define (want-array what value)
  (unless (list? value)
    (fail-input "~a is not a JSON array" what))
  value)

(define (want-string what value)
  (unless (string? value)
    (fail-input "~a is not a string" what))
  value)

(define (want-integer what value)
  (unless (exact-integer? value)
    (fail-input "~a is not an integer" what))
  value)

;; One member of a closed vocabulary, as a symbol.
(define (want-enum what value permitted)
  (define text (want-string what value))
  (define found (assoc text permitted))
  (unless found
    (fail-input "~a is `~a`, which is not one of the values protocol v1 states" what text))
  (cdr found))

(define (want-i32 what value)
  (define number (want-integer what value))
  (unless (i32? number)
    (fail-input "~a is ~a, outside the i32 range the schema states" what number))
  number)

(define (want-non-negative what value)
  (define number (want-i32 what value))
  (when (negative? number)
    (fail-input "~a is ~a, and the schema states a non-negative number" what number))
  number)

(define (want-positive what value)
  (define number (want-i32 what value))
  (unless (positive? number)
    (fail-input "~a is ~a, and the schema states a positive number" what number))
  number)

;; A byte offset. Unbounded above in the schema; bounded by the source in practice,
;; which `parse-range` checks.
(define (want-offset what value)
  (define number (want-integer what value))
  (when (negative? number)
    (fail-input "~a is ~a, and an offset is not negative" what number))
  number)

(define frames '(("full-em" . full-em) ("proportional" . proportional) ("half-em" . half-em)))
(define writing-modes '(("horizontal-tb" . horizontal-tb) ("vertical-rl" . vertical-rl)))
(define alignments '(("start" . start) ("center" . center) ("end" . end) ("justify" . justify)))
(define break-kinds '(("allowed" . allowed) ("mandatory" . mandatory) ("discretionary" . discretionary)))
(define roles
  '(("text" . text)
    ("decimal-point" . decimal-point)
    ("digit-group-separator" . digit-group-separator)
    ("sentence-medial" . sentence-medial)
    ("sentence-terminator" . sentence-terminator)
    ("grouped-numeral" . grouped-numeral)
    ("unit-symbol" . unit-symbol)
    ("quantity-symbol" . quantity-symbol)
    ("formula" . formula)
    ("warichu-bracket" . warichu-bracket)))
(define tab-alignments '(("start" . start) ("center" . center) ("end" . end) ("character" . character)))

(define (parse-size what value)
  (define object (want-object what value))
  (closed! what object '(inline block))
  (define inline (field object 'inline))
  (define block (field object 'block))
  (unless (present? inline) (fail-input "~a does not state inline" what))
  (unless (present? block) (fail-input "~a does not state block" what))
  (extent (want-positive (format "~a inline" what) inline)
          (want-positive (format "~a block" what) block)))

(define (parse-range what value limit)
  (define pieces (want-array what value))
  (unless (= (length pieces) 2)
    (fail-input "~a is not a pair of offsets" what))
  (define start (want-offset what (first pieces)))
  (define end (want-offset what (second pieces)))
  (when (> start end)
    (fail-input "~a runs backwards" what))
  (when (> end limit)
    (fail-input "~a ends at ~a, past the ~a byte(s) of source" what end limit))
  (values start end))

;; ----------------------------------------------------------------------------
;; The request
;; ----------------------------------------------------------------------------

(define (parse-request body)
  (closed! "the request"
           body
           '(source size frame clusters line_extent breaks constructs tab_stops first_line_indent
                    alignment widow_minimum_clusters writing_mode style))
  (define source (string->bytes/utf-8 (want-string "source" (require-field body 'source))))
  (define limit (bytes-length source))
  (define size (parse-size "size" (require-field body 'size)))
  (define frame (want-enum "frame" (require-field body 'frame) frames))
  (define clusters
    (for/vector ([one (in-list (want-array "clusters" (require-field body 'clusters)))]
                 [index (in-naturals)])
      (parse-cluster index one limit)))
  (define line-extent (want-positive "line_extent" (require-field body 'line_extent)))
  (define breaks
    (let ([stated (field body 'breaks)])
      (if (present? stated)
          (for/list ([one (in-list (want-array "breaks" stated))]) (parse-break one limit))
          '())))
  (define constructs
    (let ([stated (field body 'constructs)])
      (if (present? stated)
          (for/list ([one (in-list (want-array "constructs" stated))]) (parse-construct one limit))
          '())))
  (define stops
    (let ([stated (field body 'tab_stops)])
      (if (present? stated)
          (for/list ([one (in-list (want-array "tab_stops" stated))]) (parse-stop one))
          '())))
  (define indent
    (let ([stated (field body 'first_line_indent)])
      (if (present? stated) (want-i32 "first_line_indent" stated) 0)))
  (define alignment
    (let ([stated (field body 'alignment)])
      (and (present? stated) (want-enum "alignment" stated alignments))))
  (define widow
    (let ([stated (field body 'widow_minimum_clusters)])
      (and (present? stated)
           (let ([number (want-integer "widow_minimum_clusters" stated)])
             (unless (<= 1 number 65535)
               (fail-input "widow_minimum_clusters is ~a, outside [1, 65535]" number))
             number))))
  (define writing-mode
    (let ([stated (field body 'writing_mode)])
      (if (present? stated) (want-enum "writing_mode" stated writing-modes) 'horizontal-tb)))
  (define style
    (let ([stated (field body 'style)])
      (if (present? stated) stated 'jlreq-default))
    )
  (paragraph source size frame clusters line-extent breaks constructs stops indent alignment widow
             writing-mode style))

(define (require-field body name)
  (define found (field body name))
  (when (eq? found absent)
    (fail-input "the request does not state ~a" name))
  found)

(define (parse-cluster index value limit)
  (define object (want-object (format "cluster ~a" index) value))
  (closed! (format "cluster ~a" index) object '(range advance size frame role))
  (define-values (start end)
    (parse-range (format "cluster ~a range" index) (require-cluster object 'range index) limit))
  (define advance (want-non-negative (format "cluster ~a advance" index) (require-cluster object 'advance index)))
  (define size
    (let ([stated (field object 'size)])
      (and (present? stated) (parse-size (format "cluster ~a size" index) stated))))
  (define frame
    (let ([stated (field object 'frame)])
      (and (present? stated) (want-enum (format "cluster ~a frame" index) stated frames))))
  (define role
    (let ([stated (field object 'role)])
      (and (present? stated) (want-enum (format "cluster ~a role" index) stated roles))))
  (cluster start end advance size frame role))

(define (require-cluster object name index)
  (define found (field object name))
  (when (eq? found absent)
    (fail-input "cluster ~a does not state ~a" index name))
  found)

(define (parse-break value limit)
  (define object (want-object "a break" value))
  (closed! "a break" object '(offset kind))
  (define offset (want-offset "a break offset" (field-or-fail object 'offset "a break")))
  (when (> offset limit)
    (fail-input "a break is stated at ~a, past the ~a byte(s) of source" offset limit))
  (brk offset (want-enum "a break kind" (field-or-fail object 'kind "a break") break-kinds)))

(define (field-or-fail object name what)
  (define found (field object name))
  (when (eq? found absent)
    (fail-input "~a does not state ~a" what name))
  found)

(define construct-kinds
  '(("ruby" . ruby)
    ("tate-chu-yoko" . tate-chu-yoko)
    ("warichu" . warichu)
    ("formula" . formula)
    ("emphasis-dots" . emphasis-dots)
    ("furawake" . furawake)
    ("jidori" . jidori)
    ("reference-mark" . reference-mark)
    ("script" . script)))

(define (parse-construct value limit)
  (define object (want-object "a construct" value))
  (define kind (want-enum "a construct kind" (field-or-fail object 'kind "a construct") construct-kinds))
  (define-values (start end)
    (parse-range "a construct range" (field-or-fail object 'range "a construct") limit))
  (case kind
    [(tate-chu-yoko warichu formula)
     (closed! "a construct" object '(kind range))
     (construct kind start end '())]
    [(emphasis-dots)
     (closed! "a construct" object '(kind range mark))
     (construct kind start end (list (cons 'mark (want-scalar "mark" (field-or-fail object 'mark "an emphasis-dots construct")))))]
    [(jidori)
     (closed! "a construct" object '(kind range cells))
     (construct kind start end (list (cons 'cells (want-count "cells" (field-or-fail object 'cells "a jidori construct")))))]
    [(furawake)
     (closed! "a construct" object '(kind range columns line_gap))
     (construct kind
                start
                end
                (list (cons 'columns (want-count "columns" (field-or-fail object 'columns "a furawake construct")))
                      (cons 'line-gap
                            (want-non-negative "line_gap" (field-or-fail object 'line_gap "a furawake construct")))))]
    [(reference-mark script)
     (closed! "a construct" object '(kind range annotation))
     (construct kind start end (list (cons 'annotation (field-or-fail object 'annotation "the construct"))))]
    [(ruby)
     (closed! "a construct" object '(kind range ruby_kind annotation runs))
     (construct kind
                start
                end
                (list (cons 'ruby-kind (field-or-fail object 'ruby_kind "a ruby construct"))
                      (cons 'annotation (field-or-fail object 'annotation "a ruby construct"))
                      (cons 'runs (field-or-fail object 'runs "a ruby construct"))))]
    [else (fail-input "a construct names the kind `~a`" kind)]))

(define (want-scalar what value)
  (define text (want-string what value))
  (unless (= (string-length text) 1)
    (fail-input "~a is not one character" what))
  (string-ref text 0))

(define (want-count what value)
  (define number (want-integer what value))
  (unless (<= 1 number 65535)
    (fail-input "~a is ~a, outside [1, 65535]" what number))
  number)

(define (parse-stop value)
  (define object (want-object "a tab stop" value))
  (closed! "a tab stop" object '(position alignment character))
  (define position (want-positive "a tab stop position" (field-or-fail object 'position "a tab stop")))
  (define alignment (want-enum "a tab stop alignment" (field-or-fail object 'alignment "a tab stop") tab-alignments))
  (define character (field object 'character))
  (cond
    [(eq? alignment 'character)
     (when (eq? character absent)
       (fail-input "a tab stop aligns on a character and states none"))
     (tab-stop position alignment (want-scalar "a tab stop character" character))]
    [else
     (unless (eq? character absent)
       (fail-input "a tab stop states a character and does not align on one"))
     (tab-stop position alignment #f)]))

;; ----------------------------------------------------------------------------
;; Reading a cluster back out
;; ----------------------------------------------------------------------------

(define (source-slice source start end)
  (bytes->string/utf-8 (subbytes source start end) #\uFFFD))

;; The cluster's own text.
(define (cluster-text para one)
  (source-slice (paragraph-source para) (cluster-start one) (cluster-end one)))

;; The size the cluster is set at: its own where it states one, the paragraph's
;; otherwise.
(define (cluster-size-of para one)
  (or (cluster-size one) (paragraph-size para)))

;; The frame the cluster is set on, the same way.
(define (cluster-frame-of para one)
  (or (cluster-frame one) (paragraph-frame para)))
