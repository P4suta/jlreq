#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The envelope, at both ends of the wire.

(require rackunit json racket/port "../protocol.rkt")

(define (envelope #:protocol [wire protocol]
                  #:spec [revision specification]
                  #:id [id "case/one"]
                  #:request [body (hasheq)])
  (hasheq 'protocol wire 'spec revision 'id id 'request body))

(define (line-of value)
  (with-output-to-string (lambda () (write-json value))))

(module+ test
  ;; ------------------------------------------------------------------
  ;; What the two constants are
  ;; ------------------------------------------------------------------

  (check-equal? protocol "jlreq.conformance/1")
  (check-equal? specification "jlreq-2020-08-11+unicode-17.0.0")

  ;; ------------------------------------------------------------------
  ;; Reading
  ;; ------------------------------------------------------------------

  (define one (line->request (line-of (envelope))))
  (check-equal? (request-id one) "case/one")
  (check-equal? (request-body one) (hasheq))

  ;; A blank line is skipped rather than refused: it is not a request and it is not
  ;; malformed input either.
  (check-false (line->request ""))
  (check-false (line->request "   "))

  ;; Both version fields are checked, and a mismatch names what was found.
  (check-exn exn:fail? (lambda () (line->request (line-of (envelope #:protocol "jlreq.conformance/2")))))
  (check-exn exn:fail? (lambda () (line->request (line-of (envelope #:spec "jlreq-2020-08-11")))))
  (check-exn exn:fail? (lambda () (line->request (line-of (hasheq 'spec specification 'id "x" 'request (hasheq))))))
  (check-exn exn:fail? (lambda () (line->request (line-of (hasheq 'protocol protocol 'id "x" 'request (hasheq))))))

  ;; `id` is a non-empty string and `request` is an object.
  (check-exn exn:fail? (lambda () (line->request (line-of (envelope #:id "")))))
  (check-exn exn:fail? (lambda () (line->request (line-of (hasheq 'protocol protocol 'spec specification 'id 7 'request (hasheq))))))
  (check-exn exn:fail? (lambda () (line->request (line-of (hasheq 'protocol protocol 'spec specification 'id "x" 'request '())))))
  (check-exn exn:fail? (lambda () (line->request (line-of (hasheq 'protocol protocol 'spec specification 'id "x")))))

  ;; The envelope is closed. An unknown field may be the one carrying the meaning
  ;; of the request, and `expected` arriving at an engine would mean the runner is
  ;; asking the engine to grade itself.
  (check-exn exn:fail? (lambda () (line->request (line-of (hash-set (envelope) 'extra 1)))))
  (check-exn exn:fail? (lambda () (line->request (line-of (hash-set (envelope) 'expected (hasheq))))))
  (check-exn exn:fail? (lambda () (line->request (line-of (hash-set (envelope) 'rules '("3.1.10"))))))

  ;; A line that is not one JSON object is refused rather than skipped.
  (check-exn exn:fail? (lambda () (line->request "nope")))
  (check-exn exn:fail? (lambda () (line->request "[1,2]")))
  (check-exn exn:fail? (lambda () (line->request "{\"a\":1} {\"b\":2}")))

  ;; ------------------------------------------------------------------
  ;; Writing
  ;; ------------------------------------------------------------------

  ;; R0 answers every request with a complete, schema-valid, empty layout. It is
  ;; the wrong answer and the right shape; the runner reports each as a DIFF.
  (check-equal? (empty-layout) (hasheq 'lines '() 'diagnostics '()))
  (check-true (jsexpr? (answer one)))

  (define written (answer one))
  (check-equal? (hash-ref written 'protocol) protocol)
  (check-equal? (hash-ref written 'spec) specification)
  (check-equal? (hash-ref written 'id) "case/one" "the id is echoed verbatim")
  (check-equal? (hash-ref written 'response) (empty-layout))
  (check-equal? (sort (map symbol->string (hash-keys written)) string<?)
                '("id" "protocol" "response" "spec")
                "the response envelope carries these four fields and no others")

  ;; The line the engine puts on the wire ends in U+000A and holds no U+000D:
  ;; the runner splits the answer stream on the newline it wrote its requests with.
  (define emitted (with-output-to-string (lambda () (write-response (answer one) (current-output-port)))))
  (check-true (regexp-match? #rx"\n$" emitted))
  (check-false (regexp-match? #rx"\r" emitted))
  (check-equal? (length (regexp-match* #rx"\n" emitted)) 1)

  ;; And it reads back as the object it was written from, which is the comparison
  ;; the runner actually makes -- structurally, so key order is not an answer.
  (check-equal? (string->jsexpr emitted) (answer one))

  ;; Numbers never leave this engine as anything but exact integers: a single
  ;; flonum in a response is a guaranteed conformance failure, because 1 is not 1.0.
  (check-equal? (string->jsexpr (with-output-to-string
                                  (lambda () (write-json (hasheq 'advance 1000) (current-output-port)))))
                (hasheq 'advance 1000))
  (check-true (exact-integer? (hash-ref (string->jsexpr "{\"advance\":1000}") 'advance))))
