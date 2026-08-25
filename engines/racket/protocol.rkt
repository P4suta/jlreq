#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The conformance protocol: envelope in, envelope out.
;;
;; Every message the runner sends and every message the engine sends back names
;; the protocol it speaks and the revision of the specification it implements. A
;; mismatch on either is an input error and not a conformance difference: the
;; runner exits 2 on it, and so does the engine.
;;
;;   {"protocol":"jlreq.conformance/1",
;;    "spec":"jlreq-2020-08-11+unicode-17.0.0",
;;    "id":"quick-start/two-lines",
;;    "request":{...}}
;;
;; The response repeats `protocol`, `spec` and `id` and replaces `request` with
;; `response`. The runner associates unique ids in any order, so a dropped, duplicate, or unknown answer
;; is a protocol error rather than a wrong answer, and every request gets exactly
;; one line back.
;;
;; The suite files on disk carry two more fields beside `request`: `rules`, which
;; is the coverage gate's provenance, and `expected`. The runner strips both before
;; sending, so an engine never sees them and this reader refuses them -- an envelope
;; carrying `expected` into an engine would mean the runner is asking the engine to
;; grade itself.
;;
;; Closed at both ends
;;
;; protocol.schema.json is `additionalProperties: false` everywhere, and so is this
;; reader. A field the engine does not understand may be the one carrying the
;; meaning of the request, and answering it as though the field were absent
;; produces a plausible wrong answer instead of an error.
;;
;; What this module does and does not decide
;;
;; The envelope is this module's whole subject: the two version fields, the id, and
;; that `request` is an object. What is inside `request` belongs to model.rkt, which
;; reads it, and to compose.rkt, which answers it. A refusal from either is an input
;; error and reaches the runner the same way a malformed envelope does.

(require json racket/string "model.rkt" "compose.rkt")

(provide protocol
         specification
         (struct-out request)
         line->request
         envelope->request
         answer
         empty-layout
         response-envelope
         write-response)

;; The wire protocol this engine implements.
(define protocol "jlreq.conformance/1")

;; The revision of JLReq, with the Unicode version its derived tables were built
;; from. This is a specification identifier and not a version of this engine.
(define specification "jlreq-2020-08-11+unicode-17.0.0")

;; `id` is echoed verbatim in the response; `body` is the `request` object, which
;; R0 does not examine.
(struct request (id body) #:transparent)

(define (fail template . arguments)
  (raise (exn:fail (apply format template arguments) (current-continuation-marks))))

;; A sentinel for "this object has no such key", so that a JSON `false` and a
;; missing field stay distinguishable.
(define absent (string->uninterned-symbol "absent"))

(define (member* object name)
  (hash-ref object name absent))

;; ----------------------------------------------------------------------------
;; Reading
;; ----------------------------------------------------------------------------

;; One line of stdin as a request, or #f for a blank line.
(define (line->request line)
  (if (string=? (string-trim line) "")
      #f
      (envelope->request (parse-one-json line))))

;; Exactly one JSON value, and nothing after it but whitespace. `read-json` stops
;; at the end of the first value, so a line holding two of them would otherwise be
;; read as its first half.
(define (parse-one-json text)
  (define in (open-input-string text))
  (define value
    (with-handlers ([exn:fail? (lambda (raised) (fail "the line is not JSON: ~a" (exn-message raised)))])
      (read-json in)))
  (when (eof-object? value)
    (fail "the line is not JSON"))
  (let drain ()
    (define character (read-char in))
    (cond
      [(eof-object? character) (void)]
      [(char-whitespace? character) (drain)]
      [else (fail "the line carries more than one JSON value")]))
  value)

;; The one string a field must hold, or an error naming what was found instead.
(define (exact-string envelope name wanted)
  (define found (member* envelope name))
  (cond
    [(eq? found absent) (fail "the envelope does not state ~a" name)]
    [(not (string? found)) (fail "~a is not a string" name)]
    [(not (string=? found wanted))
     (fail "~a is `~a`, and this engine speaks `~a`" name found wanted)]
    [else (void)]))

;; Read one request envelope. The envelope is closed: `protocol`, `spec`, `id` and
;; `request`, in any order, and nothing else.
(define (envelope->request envelope)
  (unless (hash? envelope)
    (fail "the message is not a JSON object"))
  (exact-string envelope 'protocol protocol)
  (exact-string envelope 'spec specification)
  (define id
    (let ([found (member* envelope 'id)])
      (cond
        [(eq? found absent) (fail "the envelope does not state id")]
        [(not (string? found)) (fail "id is not a string")]
        [(string=? found "") (fail "id is empty")]
        [else found])))
  (define body
    (let ([found (member* envelope 'request)])
      (cond
        [(eq? found absent) (fail "the envelope does not state request")]
        [(not (hash? found)) (fail "request is not a JSON object")]
        [else found])))
  (for ([name (in-list (hash-keys envelope))])
    (case name
      [(protocol spec id request) (void)]
      [(expected) (fail "the envelope states expected, which the runner strips before sending")]
      [else (fail "the envelope states `~a`, which protocol v1 does not carry" name)]))
  (request id body))

;; ----------------------------------------------------------------------------
;; Writing
;; ----------------------------------------------------------------------------

;; The response a paragraph with nothing on any line produces: no lines and no
;; diagnostics. A request whose `clusters` array is empty is answered with this,
;; which is a complete, schema-valid answer rather than an absent one.
(define (empty-layout)
  (hasheq 'lines '() 'diagnostics '()))

;; Answer one request: read the body, compose it, and put the layout back in the
;; envelope the id came in.
(define (answer one)
  (define-values (lines notes) (compose (parse-request (request-body one))))
  (response-envelope (request-id one) (layout->json lines notes)))

;; The layout, as the schema's own two arrays.
(define (layout->json lines notes)
  (hasheq 'lines (for/list ([one (in-list lines)]) (line->json one))
          'diagnostics (for/list ([one (in-list notes)]) (diagnostic->json one))))

(define (line->json one)
  (hasheq 'range (list (line-start one) (line-end one))
          'inline_origin (line-inline-origin one)
          'block_origin (line-block-origin one)
          'inline_extent (line-inline-extent one)
          'block_extent (line-block-extent one)
          'clusters (for/list ([each (in-list (line-clusters one))]) (placement->json each))
          'attachments (for/list ([each (in-list (line-attachments one))]) (attachment->json each))))

;; An annotation carries either its own shaped text -- a ruby reading, a
;; superscript -- or one symbol repeated over a base character, and `symbol` is
;; which. The range is a range of the ANNOTATION's own source in the first case, so
;; nothing here reads it against the paragraph.
(define (attachment->json one)
  (hasheq 'construct (attached-construct one)
          'range (list (attached-start one) (attached-end one))
          'inline (attached-inline one)
          'block (attached-block one)
          'advance (attached-advance one)
          'size (hasheq 'inline (extent-inline (attached-size one))
                        'block (extent-block (attached-size one)))
          'writing_mode (writing-mode->json (attached-writing-mode one))
          'transform (transform->json (attached-transform one))
          'symbol (let ([found (attached-symbol one)])
                    (if found (string found) (json-null)))))

(define (placement->json one)
  (hasheq 'origin (hasheq 'cluster (placed-index one))
          'range (list (placed-start one) (placed-end one))
          'inline (placed-inline one)
          'block (placed-block one)
          'advance (placed-advance one)
          'size (hasheq 'inline (extent-inline (placed-size one))
                        'block (extent-block (placed-size one)))
          'frame (frame->json (placed-frame one))
          'writing_mode (writing-mode->json (placed-writing-mode one))
          'transform (transform->json (placed-transform one))))

;; A diagnostic is (code severity start end address), with `start` #f where it names
;; no range of the source.
(define (diagnostic->json one)
  (hasheq 'code (list-ref one 0)
          'severity (list-ref one 1)
          'range (if (list-ref one 2) (list (list-ref one 2) (list-ref one 3)) (json-null))
          'jlreq (list-ref one 4)))

(define (frame->json value)
  (case value
    [(full-em) "full-em"]
    [(proportional) "proportional"]
    [else "half-em"]))

(define (writing-mode->json value)
  (if (eq? value 'vertical-rl) "vertical-rl" "horizontal-tb"))

(define (transform->json value)
  (case value
    [(rotate-clockwise) "rotate-clockwise"]
    [(tate-chu-yoko) "tate-chu-yoko"]
    [else "identity"]))

(define (response-envelope id response)
  (hasheq 'protocol protocol 'spec specification 'id id 'response response))

;; One envelope and the newline the runner splits on. Nothing here writes a
;; carriage return: line endings are the engine's to choose, and a `\r` the runtime
;; inserted would be a byte the engine did not decide to write.
(define (write-response envelope out)
  (write-json envelope out)
  (write-char #\newline out))
