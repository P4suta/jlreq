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
;; `response`. The runner checks the ids in order, so a dropped or reordered answer
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
;; Where R0 stops
;;
;; This milestone reads the envelope and nothing inside `request`. Every request is
;; answered with an empty layout, which the runner reports as eighty-nine DIFFs and
;; exit 1 -- the engine is being written to fix those, one milestone at a time, and
;; a DIFF is not a protocol failure. What R0 does claim is the transport: the
;; envelope is validated, the response validates against the schema, and a
;; malformed line is refused rather than skipped.

(require json racket/string)

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

;; The response a paragraph that composes to nothing produces: no lines and no
;; diagnostics. It is a complete, schema-valid response, which is the point --
;; R0's answer is wrong and well formed, not absent.
(define (empty-layout)
  (hasheq 'lines '() 'diagnostics '()))

;; Answer one request. R0 answers every request the same way; the milestones after
;; it are what make this read `request-body`.
(define (answer one)
  (response-envelope (request-id one) (empty-layout)))

(define (response-envelope id response)
  (hasheq 'protocol protocol 'spec specification 'id id 'response response))

;; One envelope and the newline the runner splits on. Nothing here writes a
;; carriage return: line endings are the engine's to choose, and a `\r` the runtime
;; inserted would be a byte the engine did not decide to write.
(define (write-response envelope out)
  (write-json envelope out)
  (write-char #\newline out))
