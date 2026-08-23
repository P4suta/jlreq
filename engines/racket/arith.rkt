#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; Integer semantics.
;;
;; The contract this module implements is stated once, in "The integer contract"
;; of docs/design/conformance.md, and is not restated here: every reference
;; engine names that section rather than re-deriving the reasoning locally, so
;; the contract is amended in exactly one place. What follows is only what is
;; specific to Racket.
;;
;; Racket's exact integers are unbounded. That is the opposite problem from
;; OCaml's, whose native `int` is 63 bits and therefore *narrower* than the i64
;; layer the contract names: here nothing ever overflows on its own, so no
;; saturation happens unless this module puts it there. Every bound below is an
;; explicit clamp applied to an exact result, which is the same answer Rust's
;; `saturating_*` gives and is arrived at without a wraparound test -- `(+ a b)`
;; on two i64s is already the mathematically correct sum, so clamping it is the
;; definition of saturating addition rather than a repair of one.
;;
;; The three layers:
;;
;;   i64     the width intermediate sums, products and the line-breaking cost
;;           function are computed at. `sat+`, `sat-`, `sat*`, `sat-neg` and
;;           `sat-abs` saturate at the i64 bounds; `chk+`, `chk-` and `chk*`
;;           refuse to leave them, which the engine turns into exit 2.
;;   i32     the width every number that crosses the protocol boundary has.
;;           `clamp-i32` is the only way an i64-layer computation becomes one.
;;   usize   a byte offset, or a remaining width. `usub` stops at zero; Racket's
;;           `-` does not, and a negative offset is a wrong answer rather than a
;;           crash, so every difference of two offsets goes through here.
;;
;; `quotient` and `remainder` already truncate toward zero, which is what Rust's
;; `/` and `%` do: `(quotient -7 2)` is -3 and `(remainder -7 2)` is -1.
;; `div-trunc` and `rem-trunc` exist to say so at the call site. Racket's
;; `floor`, `/` on non-multiples, and anything that can produce a flonum or an
;; exact rational are banned everywhere in this engine: a single non-integer in a
;; response is a guaranteed conformance failure, because comparison is structural
;; JSON equality and 1 is never 1.0.

(provide i64-max
         i64-min
         i64?
         clamp-i64
         sat+
         sat-
         sat*
         sat-neg
         sat-abs
         chk+
         chk-
         chk*
         infinite-cost
         i32-max
         i32-min
         i32?
         clamp-i32
         i32+
         i32-
         div-trunc
         rem-trunc
         usub
         uadd
         exact-integer-or-fail)

;; ----------------------------------------------------------------------------
;; The i64 layer
;; ----------------------------------------------------------------------------

;; i64::MAX.
(define i64-max 9223372036854775807)

;; i64::MIN.
(define i64-min -9223372036854775808)

;; Whether `value` would survive the i64 layer unchanged.
(define (i64? value)
  (and (exact-integer? value) (<= i64-min value i64-max)))

;; An exact integer brought into i64 range by saturation.
(define (clamp-i64 value)
  (cond
    [(< value i64-min) i64-min]
    [(> value i64-max) i64-max]
    [else value]))

;; i64::saturating_add, and the two below are `saturating_sub` and
;; `saturating_mul`. The sum is exact before it is clamped, so there is no
;; wrapped intermediate to detect the overflow from.
(define (sat+ a b) (clamp-i64 (+ (exact-integer-or-fail 'sat+ a) (exact-integer-or-fail 'sat+ b))))
(define (sat- a b) (clamp-i64 (- (exact-integer-or-fail 'sat- a) (exact-integer-or-fail 'sat- b))))
(define (sat* a b) (clamp-i64 (* (exact-integer-or-fail 'sat* a) (exact-integer-or-fail 'sat* b))))

;; i64::saturating_neg and i64::saturating_abs: i64::MIN becomes i64::MAX,
;; which falls out of clamping the exact negation rather than being a case.
(define (sat-neg a) (clamp-i64 (- (exact-integer-or-fail 'sat-neg a))))
(define (sat-abs a) (clamp-i64 (abs (exact-integer-or-fail 'sat-abs a))))

;; The checked forms. A computation that leaves the i64 range is a fault in this
;; engine rather than a number to be rounded off, so these raise; `main.rkt`
;; turns the failure into one line on stderr and exit 2.
(define (checked who value)
  (unless (i64? value)
    (raise (exn:fail (format "~a: ~a is outside the 64 bit range this engine computes in" who value)
                     (current-continuation-marks))))
  value)

(define (chk+ a b) (checked 'chk+ (+ (exact-integer-or-fail 'chk+ a) (exact-integer-or-fail 'chk+ b))))
(define (chk- a b) (checked 'chk- (- (exact-integer-or-fail 'chk- a) (exact-integer-or-fail 'chk- b))))
(define (chk* a b) (checked 'chk* (* (exact-integer-or-fail 'chk* a) (exact-integer-or-fail 'chk* b))))

;; The cost the line breaker treats as unreachable.
;;
;; i64::MAX / 4 rather than i64::MAX, so that a handful of these can be added
;; together without the sum saturating and losing the ordering between two
;; equally impossible breaks.
(define infinite-cost (quotient i64-max 4))

;; ----------------------------------------------------------------------------
;; The i32 layer
;; ----------------------------------------------------------------------------

;; i32::MAX and i32::MIN, the range protocol.schema.json states for every number
;; that crosses the boundary.
(define i32-max 2147483647)
(define i32-min -2147483648)

;; Whether `value` would survive the protocol's i32 range unchanged.
(define (i32? value)
  (and (exact-integer? value) (<= i32-min value i32-max)))

;; An exact integer brought into i32 range by saturation. This is the only way an
;; i64-layer computation becomes a number the protocol can carry.
(define (clamp-i32 value)
  (cond
    [(< value i32-min) i32-min]
    [(> value i32-max) i32-max]
    [else value]))

(define (i32+ a b) (clamp-i32 (+ (exact-integer-or-fail 'i32+ a) (exact-integer-or-fail 'i32+ b))))
(define (i32- a b) (clamp-i32 (- (exact-integer-or-fail 'i32- a) (exact-integer-or-fail 'i32- b))))

;; ----------------------------------------------------------------------------
;; Division
;; ----------------------------------------------------------------------------

;; Division truncated toward zero, which is what Rust's `/` does: -7 / 2 is -3
;; and not -4. The engine never wants a floor. `quotient` raises on a zero
;; divisor, matching the Rust panic.
(define (div-trunc a b) (quotient (exact-integer-or-fail 'div-trunc a) (exact-integer-or-fail 'div-trunc b)))

;; Remainder with the sign of the dividend, which is what Rust's `%` does:
;; -7 % 2 is -1 and not 1. Racket's `modulo` is the other one and is never used.
(define (rem-trunc a b) (remainder (exact-integer-or-fail 'rem-trunc a) (exact-integer-or-fail 'rem-trunc b)))

;; ----------------------------------------------------------------------------
;; The usize layer
;; ----------------------------------------------------------------------------

;; Saturating subtraction on a byte offset: the result is never negative.
;; Rust's usize::saturating_sub stops at zero. Racket's `-` does not.
(define (usub a b)
  (let ([a (exact-integer-or-fail 'usub a)] [b (exact-integer-or-fail 'usub b)])
    (if (> a b) (- a b) 0)))

;; Addition on a byte offset. Racket's integers are unbounded, so this cannot
;; overflow and there is nothing to saturate at; it exists so that the intent
;; reads the same as `usub`'s at the call site.
(define (uadd a b)
  (+ (exact-integer-or-fail 'uadd a) (exact-integer-or-fail 'uadd b)))

;; ----------------------------------------------------------------------------
;; The float ban
;; ----------------------------------------------------------------------------

;; `value` if it is an exact integer, and an error otherwise.
;;
;; Every arithmetic entry point above runs its arguments through this. A flonum
;; or an exact rational reaching a response is a guaranteed conformance failure
;; that a structural comparison reports as a wrong *answer*, which is the hardest
;; kind of defect to find; refusing it here turns it into one line on stderr.
(define (exact-integer-or-fail who value)
  (unless (exact-integer? value)
    (raise (exn:fail (format "~a: ~s is not an exact integer, and this engine computes in nothing else"
                             who
                             value)
                     (current-continuation-marks))))
  value)
