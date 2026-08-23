#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The integer contract of docs/design/conformance.md, tested at its edges.
;;
;; Racket's integers are unbounded, so every bound these check is one arith.rkt put
;; there. That is the whole risk this file exists for: an engine whose arithmetic
;; silently carries on past i64::MAX agrees with Rust everywhere the numbers are
;; small and disagrees exactly where the line breaker's sentinel cost lives.

(require rackunit "../arith.rkt")

(module+ test
  ;; ------------------------------------------------------------------
  ;; The i64 layer
  ;; ------------------------------------------------------------------

  (check-equal? i64-max 9223372036854775807)
  (check-equal? i64-min -9223372036854775808)
  (check-true (i64? i64-max))
  (check-true (i64? i64-min))
  (check-false (i64? (add1 i64-max)))
  (check-false (i64? (sub1 i64-min)))

  (check-equal? (sat+ 2 3) 5)
  (check-equal? (sat+ i64-max 1) i64-max "saturating_add stops at i64::MAX")
  (check-equal? (sat+ i64-max i64-max) i64-max)
  (check-equal? (sat+ i64-min -1) i64-min)
  (check-equal? (sat- 2 3) -1)
  (check-equal? (sat- i64-min 1) i64-min "saturating_sub stops at i64::MIN")
  (check-equal? (sat- i64-max -1) i64-max)
  (check-equal? (sat* 6 7) 42)
  (check-equal? (sat* i64-max 2) i64-max)
  (check-equal? (sat* i64-max -2) i64-min)
  (check-equal? (sat* i64-min -1) i64-max "saturating_mul: -i64::MIN has no i64")
  (check-equal? (sat* 0 i64-min) 0)

  (check-equal? (sat-neg 5) -5)
  (check-equal? (sat-neg i64-min) i64-max "saturating_neg: i64::MIN becomes i64::MAX")
  (check-equal? (sat-abs -5) 5)
  (check-equal? (sat-abs i64-min) i64-max)

  (check-equal? (chk+ 2 3) 5)
  (check-exn exn:fail? (lambda () (chk+ i64-max 1)) "the checked forms refuse to leave the range")
  (check-exn exn:fail? (lambda () (chk- i64-min 1)))
  (check-exn exn:fail? (lambda () (chk* i64-max 2)))

  ;; i64::MAX / 4, so that several of these can be added without the sum
  ;; saturating and losing the ordering between two equally impossible breaks.
  (check-equal? infinite-cost (quotient i64-max 4))
  (check-true (i64? (sat+ infinite-cost (sat+ infinite-cost infinite-cost))))
  (check-true (< (sat+ infinite-cost infinite-cost) i64-max))

  ;; ------------------------------------------------------------------
  ;; The i32 layer
  ;; ------------------------------------------------------------------

  (check-equal? i32-max 2147483647)
  (check-equal? i32-min -2147483648)
  (check-true (i32? 0))
  (check-false (i32? (add1 i32-max)))
  (check-equal? (clamp-i32 (add1 i32-max)) i32-max)
  (check-equal? (clamp-i32 (sub1 i32-min)) i32-min)
  (check-equal? (clamp-i32 1000) 1000)
  (check-equal? (i32+ i32-max 1) i32-max)
  (check-equal? (i32- i32-min 1) i32-min)
  (check-equal? (i32+ 1000 720) 1720)

  ;; ------------------------------------------------------------------
  ;; Division truncates toward zero
  ;; ------------------------------------------------------------------

  (check-equal? (div-trunc -7 2) -3 "Rust's / truncates; a floor would give -4")
  (check-equal? (div-trunc 7 -2) -3)
  (check-equal? (rem-trunc -7 2) -1 "Rust's % takes the sign of the dividend")
  (check-equal? (rem-trunc 7 -2) 1)
  (check-equal? (div-trunc 1665 2) 832)
  (check-exn exn:fail? (lambda () (div-trunc 1 0)))

  ;; ------------------------------------------------------------------
  ;; The usize layer
  ;; ------------------------------------------------------------------

  (check-equal? (usub 5 3) 2)
  (check-equal? (usub 3 5) 0 "a byte offset minus a byte offset never goes negative")
  (check-equal? (usub 0 0) 0)
  (check-equal? (uadd 3 5) 8)

  ;; ------------------------------------------------------------------
  ;; The float ban
  ;; ------------------------------------------------------------------

  (check-exn exn:fail? (lambda () (sat+ 1.0 1)) "a flonum never reaches a response")
  (check-exn exn:fail? (lambda () (i32+ 1 1.5)))
  (check-exn exn:fail? (lambda () (div-trunc 1/2 1)) "an exact rational is not an integer either")
  (check-exn exn:fail? (lambda () (usub "1" 1))))
