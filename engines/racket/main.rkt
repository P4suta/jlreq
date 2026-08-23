#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The engine process.
;;
;; `jlreq-conformance run` starts this executable once, with no arguments and no
;; environment it tells us about, writes every request to stdin as one JSON object
;; per line, closes stdin, and reads one response line per request after the
;; process exits. There is no handshake and no way to ask a question back.
;;
;; Exit codes are the protocol's:
;;
;; - 0, every request was answered;
;; - 2, the specification tables did not come out right, a line was not JSON, or an
;;   envelope named a protocol or a specification this engine does not speak.
;;
;; A wrong ANSWER is not an error here. It is reported by the runner as a DIFF and
;; exits the RUNNER 1. At R0 every one of the eighty-nine cases is such a DIFF.

(require "protocol.rkt" (prefix-in tables: "tables.rkt"))

(define program "jlreq-engine-racket")

(define (die message)
  (eprintf "~a: ~a\n" program message)
  (exit 2))

;; Answer every line of stdin.
(define (serve)
  (define in (current-input-port))
  (define out (current-output-port))
  (let loop ()
    ;; 'linefeed and not 'any: the runner splits the answer stream on U+000A and
    ;; writes its own requests the same way, so U+000D is data here rather than a
    ;; terminator. A JSON string cannot carry a raw one -- the format escapes every
    ;; control character -- so nothing is lost by not looking for it.
    (define line (read-line in 'linefeed))
    (unless (eof-object? line)
      (define one (line->request line))
      (when one
        (write-response (answer one) out))
      (loop))))

(module+ main
  ;; The tables are built when tables.rkt is instantiated, which has already
  ;; happened by the time this runs; the census is what decides whether what was
  ;; built is what `spec/` states.
  (with-handlers ([exn:fail? (lambda (raised) (die (format "specification tables: ~a" (exn-message raised))))])
    (tables:self-check))
  (with-handlers ([exn:fail? (lambda (raised) (die (exn-message raised)))])
    (serve))
  (flush-output))
