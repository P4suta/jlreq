#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; §3.9.2, at the keys the published readings are about.
;;
;; Appendix A names 473 of its 1,133 keys under more than one class, so most of what
;; `classify` does is decide between listings rather than look one up. The keys
;; below are the ones the readings in `docs/decisions/` and the "Observable policies"
;; section of `engines/ocaml/README.md` argue over, plus the two that tell the two
;; readings of a Remarks cell naming an inexpressible width apart -- U+0020 and
;; U+2010. A regression in the tie-break shows up here as a named key rather than as
;; a spacing difference nine coordinates away.

(require rackunit "../classes.rkt" "../style.rkt")

(define jlreq (resolve-style "jlreq-2020"))

(define (class-of text frame [role #f] [writing-mode 'horizontal-tb] [style jlreq])
  (classify (folded-key text) frame writing-mode role style))

(module+ test
  ;; ------------------------------------------------------------------
  ;; The frame decides between a Japanese design and a Western one
  ;; ------------------------------------------------------------------

  ;; §3.9.2's own example: a parenthesis is cl-01 in Japanese text and cl-27 set
  ;; proportionally, and §3.1.2's half-width character advance is why the frame is
  ;; what separates them.
  (check-equal? (class-of "(" 'full-em) 1)
  (check-equal? (class-of "(" 'half-em) 1)
  (check-equal? (class-of "(" 'proportional) 27)
  (check-equal? (class-of ")" 'full-em) 2)
  (check-equal? (class-of ")" 'proportional) 27)
  (check-equal? (class-of "," 'half-em) 7)
  (check-equal? (class-of "," 'proportional) 27)

  ;; §3.2.4 and §3.2.6: a Western character is quasi-Japanese on the full em and on
  ;; the fixed width, and Western when it is set proportionally.
  (check-equal? (class-of "A" 'full-em) 19)
  (check-equal? (class-of "A" 'half-em) 19)
  (check-equal? (class-of "A" 'proportional) 27)
  (check-equal? (class-of "1" 'full-em) 19)
  (check-equal? (class-of "1" 'half-em) 19 "the half em is §3.2.4's fixed width, not §A.24's construct")
  (check-equal? (class-of "1" 'proportional) 27)

  ;; A class with no Japanese advance of its own keeps the frame out of it.
  (check-equal? (class-of "!" 'proportional) 4)
  (check-equal? (class-of "#" 'proportional) 12)
  (check-equal? (class-of "%" 'proportional) 13)

  ;; ------------------------------------------------------------------
  ;; A Remarks cell naming a width the protocol cannot express
  ;; ------------------------------------------------------------------

  ;; U+0020 is a grouped numeral's space and a unit symbol's at a quarter em and the
  ;; Western word space unqualified, so cl-26 stands however it is measured.
  (check-equal? (class-of " " 'full-em) 26)
  (check-equal? (class-of " " 'half-em) 26)
  (check-equal? (class-of " " 'proportional) 26)

  ;; U+2010 is a hyphen at a quarter em and a Western character proportional. Where
  ;; the frame reaches neither, every listing stands again and the lowest survivor
  ;; is the answer.
  (check-equal? (class-of "‐" 'full-em) 3)
  (check-equal? (class-of "‐" 'half-em) 3)
  (check-equal? (class-of "‐" 'proportional) 27)

  ;; A key only §A.27 lists is cl-27 on every frame, because an empty candidate set
  ;; is not an answer.
  (check-equal? (class-of "\"" 'full-em) 27)
  (check-equal? (class-of "\"" 'half-em) 27)

  ;; ------------------------------------------------------------------
  ;; Folding
  ;; ------------------------------------------------------------------

  ;; Appendix A keys the narrow form and real text carries the wide one.
  (check-equal? (class-of "）" 'full-em) 2 "U+FF09 folds onto U+0029")
  (check-equal? (class-of "（" 'full-em) 1)
  ;; ... but not over a key the appendix already lists: §A.14's whole enumeration is
  ;; the Wide decomposition source of U+0020.
  (check-equal? (class-of "　" 'full-em) 14 "U+3000 is cl-14 and not a folded space")

  ;; ------------------------------------------------------------------
  ;; The declared role
  ;; ------------------------------------------------------------------

  (check-equal? (class-of "1" 'half-em 'grouped-numeral) 24)
  (check-equal? (class-of "1" 'half-em 'decimal-point) 24)
  (check-equal? (class-of "1" 'half-em 'digit-group-separator) 24)
  (check-equal? (class-of "1" 'half-em 'unit-symbol) 25)
  (check-equal? (class-of "1" 'full-em 'grouped-numeral) 19 "§A.24's cell names the half em")
  (check-equal? (class-of "(" 'full-em 'warichu-bracket) 28)
  (check-equal? (class-of ")" 'full-em 'warichu-bracket) 29)
  (check-equal? (class-of "A" 'full-em 'unit-symbol) 19 "§A.25 lists U+0041 proportional only")
  (check-equal? (class-of "A" 'proportional 'unit-symbol) 25)
  (check-equal? (class-of " " 'full-em 'grouped-numeral) 26 "a quarter em is a width no caller declares")

  ;; ------------------------------------------------------------------
  ;; The writing mode
  ;; ------------------------------------------------------------------

  ;; §A.06 lists U+002E "used in horizontal composition", so a vertical line's full
  ;; stop is not that listing at all: what is left describing the occurrence is
  ;; §A.24's decimal point.
  (check-equal? (class-of "." 'half-em #f 'horizontal-tb) 6)
  (check-equal? (class-of "." 'full-em #f 'horizontal-tb) 6)
  (check-equal? (class-of "." 'half-em #f 'vertical-rl) 24)
  (check-equal? (class-of "." 'full-em #f 'vertical-rl) 24)
  (check-equal? (class-of "." 'proportional #f 'vertical-rl) 27)
  (check-equal? (class-of "," 'full-em #f 'vertical-rl) 24)
  ;; §A.08 lists the vertical kana repeat marks in vertical composition alone, and
  ;; a horizontal line still has to answer for one.
  (check-equal? (class-of "〳" 'full-em #f 'horizontal-tb) 8)
  (check-equal? (class-of "〳" 'full-em #f 'vertical-rl) 8)

  ;; ------------------------------------------------------------------
  ;; The three questions §3.9.2 leaves open
  ;; ------------------------------------------------------------------

  ;; docs/decisions/unlisted-code-point.md
  (check-equal? (class-of "\U0001F980" 'full-em) 19)
  (check-equal? (class-of "\U0001F980" 'half-em) 19)
  (check-equal? (class-of "\U0001F980" 'proportional) 27)
  (let ([ideographic (resolve-style (hasheq 'profile "jlreq-2020"
                                            'classification.unlisted_code_point "ideographic"))])
    (check-equal? (class-of "\U0001F980" 'proportional #f 'horizontal-tb ideographic) 19))

  ;; docs/decisions/ambiguous-context.md: U+2194 is listed under cl-17 and cl-19.
  (check-equal? (class-of "↔" 'full-em) 17)
  (let ([highest (resolve-style (hasheq 'profile "jlreq-2020"
                                        'classification.ambiguous_context "highest-class"))])
    (check-equal? (class-of "↔" 'full-em #f 'horizontal-tb highest) 19))

  ;; docs/decisions/grouped-numeral-qualification.md
  (let ([by-role (resolve-style (hasheq 'profile "jlreq-2020"
                                        'classification.grouped_numeral_qualification "by-role"))])
    (check-equal? (class-of "1" 'half-em #f 'horizontal-tb by-role) 27
                  "the width admits §A.24 and the caller declared no job")
    (check-equal? (class-of "1" 'half-em 'grouped-numeral 'horizontal-tb by-role) 24)
    (check-equal? (class-of "1" 'full-em #f 'horizontal-tb by-role) 19
                  "the width does not admit §A.24, so the question does not arise"))

  ;; ------------------------------------------------------------------
  ;; The predicates the derived tables carry
  ;; ------------------------------------------------------------------

  (check-true (unified-ideograph? #x65E5))
  (check-true (unified-ideograph? #xFA0E) "a compatibility ideograph with no decomposition")
  (check-false (unified-ideograph? #xFA10) "one that decomposes is not Unified_Ideograph")
  (check-equal? (script-of #x3042) "Hiragana")
  (check-equal? (script-of #x30A2) "Katakana")
  (check-equal? (script-of #x30FC) #f "the prolonged sound mark is Script=Common")
  (check-equal? (script-of #x30FD) "Katakana")

  (check-true (construct-class? 24))
  (check-true (construct-class? 30))
  (check-false (construct-class? 19))
  (check-false (construct-class? 27)))
