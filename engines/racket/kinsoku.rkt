#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; Appendix C: whether a line may be broken between two occurrences.
;;
;; Table 2 is the base answer, in three values: a blank cell is a break opportunity,
;; `not` is none, and `×` is a combination that does not occur at all — every one of
;; the twenty is a warichu bracket beside something that cannot stand beside it.
;;
;; Four of the §C.2 notes qualify a cell rather than restate it, and each is a fact
;; about the two *occurrences* that a class pair cannot carry:
;;
;; - note 5, at (cl-08, cl-08): the cell is blank, and the note names five ordered
;;   couples that are nonetheless one unit — two em dashes, two horizontal
;;   ellipses, two two-dot leaders, and the two kunojiten crossings, upper half
;;   then lower half. It is an ordered-pair list and is read as one: a mark is not
;;   held to a mark the note never pairs it with. (§E.2 note 4 asks a different
;;   question about the same class — "of different kinds" — and
;;   `docs/decisions/inseparable-character-kind.md` answers that one with a
;;   partition rather than with this list.)
;; - note 9, at (cl-24, cl-13): the cell is `not`, and §C.3's `loose` level names
;;   PERCENT SIGN as the one member of cl-13 a line may break before.
;; - note 10, at (cl-24, cl-27): the cell is blank, and
;;   `kinsoku.grouped_numeral_before_western` is the question of whether it stays so.
;; - note 11, at (cl-27, cl-13): the cell is blank, and the note withdraws the
;;   opportunity where the Western character "is used as a symbol of a quantity or a
;;   European numeral". The first is the caller's `quantity-symbol` role; the second
;;   is read from the occurrence's own key, because no caller declares it
;;   (`docs/decisions/european-numeral-by-code-point.md`).
;;
;; §C.3's four levels, and the two mechanisms
;;
;; The addendum grades the prohibitions rather than restating Table 2, and two
;; prohibitions are outside the grading entirely: a break after an opening bracket
;; and a break before a closing bracket, a full stop or a comma stay prohibited at
;; every level, which the section states in its own opening paragraph.
;;
;; `very-loose` — the newspaper level — is those two and nothing else. The section
;; lists nine classes there, and the list is not a smaller claim than that: between
;; two ordinary characters every remaining `not` cell of Table 2 is one of the nine's
;; own coordinates, so "everything but the universal prohibitions" and "the nine
;; classes" name the same set of boundaries.
;;
;; `loose` and `strict` are where the two mechanisms `kinsoku.relaxation_mechanism`
;; names come apart, and the difference is visible:
;;
;; - `reclassify` (§B.2 notes 14 through 16, §C.2 notes 1 through 3): a prolonged
;;   sound mark *becomes* katakana, a small kana becomes hiragana or katakana by its
;;   own script, and an iteration mark becomes an ideograph — and Table 2 is then
;;   asked at the new coordinate. A prefixed abbreviation before a prolonged sound
;;   mark is still unbreakable under this mechanism, because (cl-12, cl-16) is `not`
;;   for a reason that has nothing to do with the sound mark.
;; - `matrix`: the prohibition is lifted at the boundary and the class is left
;;   alone, so the same pair does break.
;;
;; The iteration mark is the one of the three §C.3 relaxes and a Style question also
;; governs: `kinsoku.iteration_mark_at_line_head` publishes §B.2 note 14's own three
;; ways, and its default answer — `prohibited`, "follow the principle by applying
;; some sort of line adjustment" — is what keeps a strict line from breaking before
;; 々 even though §C.3's `strict` level names it.

(require (prefix-in tables: "tables.rkt")
         "classes.rkt"
         "spacing.rkt"
         "style.rkt"
         "model.rkt")

(provide breakable?
         kinsoku-level
         european-numeral?
         quantity-or-numeral?
         reclassified
         inseparable-kind
         single-key)

(define table2 (tables:matrix-of 2))

;; §C.3's levels as ordinals, the order the section lists them in.
(define (kinsoku-level style)
  (define chosen (answer style "kinsoku.level"))
  (cond
    [(string=? chosen "very-loose") 1]
    [(string=? chosen "loose") 2]
    [(string=? chosen "strict") 3]
    [else 4]))

;; The code points §C.3 and §C.2 note 5 name one at a time.
(define ideographic-iteration-mark #x3005)
(define percent-sign #x0025)
(define katakana-middle-dot #x30FB)
(define horizontal-ellipsis #x2026)
(define two-dot-leader #x2025)
(define em-dash #x2014)
(define kana-repeat-upper #x3033)
(define kana-repeat-voiced-upper #x3034)
(define kana-repeat-lower #x3035)

;; The occurrence's folded key, or #f where it is not one code point.
(define (single-key para one)
  (define key (folded-key (source-slice (paragraph-source para) (item-start one) (item-end one))))
  (and (= (length key) 1) (car key)))

;; §C.2 note 11's "European numeral", read from the key: the ten §A.19, §A.24 and
;; §A.27 all enumerate under that name, and nothing else.
(define (european-numeral? scalar)
  (and scalar (<= #x0030 scalar #x0039) #t))

;; §E.2 note 10 and §C.2 note 11 name the same two occurrences: a Western character
;; (cl-27) the caller declared a symbol of a quantity, and one of the ten European
;; numerals. The first refuses the break and the second refuses the expansion.
(define (quantity-or-numeral? para one)
  (or (eq? (item-role one) 'quantity-symbol)
      (european-numeral? (single-key para one))))

;; §C.2 note 5's own five ordered couples, which is a shorter list than §E.2 note
;; 4's own partition and is not the same statement.
(define (indivisible-couple? before after)
  (and before
       after
       (or (and (= before em-dash) (= after em-dash))
           (and (= before horizontal-ellipsis) (= after horizontal-ellipsis))
           (and (= before two-dot-leader) (= after two-dot-leader))
           (and (= before kana-repeat-upper) (= after kana-repeat-lower))
           (and (= before kana-repeat-voiced-upper) (= after kana-repeat-lower)))))

;; §E.2 note 4's "kind": four among cl-08's six members, the three kunojiten marks
;; together (`docs/decisions/inseparable-character-kind.md`).
(define (inseparable-kind scalar)
  (cond
    [(not scalar) #f]
    [(memv scalar (list kana-repeat-upper kana-repeat-voiced-upper kana-repeat-lower)) 'kunojiten]
    [else scalar]))

;; ----------------------------------------------------------------------------
;; Reclassification
;; ----------------------------------------------------------------------------

;; The class an occurrence takes once §C.3's own level has let it start a line.
;;
;; This is a change of *class*, not of one boundary's answer: §B.2 notes 15 and 16
;; say the character "shall be treated as part of" the other class, so every table
;; is read at the new coordinate afterwards.
(define (reclassified para style one value)
  (cond
    [(>= (kinsoku-level style) 4) value]
    [(not (answer-is? style "kinsoku.relaxation_mechanism" "reclassify")) value]
    [(= value 10) 16]
    [(= value 11)
     (define script (let ([key (single-key para one)]) (and key (script-of key))))
     (cond
       [(equal? script "Hiragana") 15]
       [(equal? script "Katakana") 16]
       [else value])]
    [(and (= value 9)
          (eqv? (single-key para one) ideographic-iteration-mark)
          (not (answer-is? style "kinsoku.iteration_mark_at_line_head" "prohibited")))
     19]
    [else value]))

;; ----------------------------------------------------------------------------
;; §C.3's relaxations
;; ----------------------------------------------------------------------------

;; The two prohibitions §C.3 states are common to every level and lists nowhere.
(define (always-prohibited? before-class after-class)
  (or (= before-class 1) (= after-class 2) (= after-class 6) (= after-class 7)))

;; A class whose indivisibility is structural rather than one of §C.3's own
;; conventions: what a character *becomes* inside a construct. §C.2 notes 6, 7, 8
;; and 13 state those, and no level of the addendum grades them.
(define (structural? value)
  (or (<= 20 value 23) (= value 28) (= value 29) (= value 30)))

;; Whether the level lifts the prohibition at this boundary.
(define (relaxed? style level before after before-class after-class before-key after-key)
  (define matrix? (not (answer-is? style "kinsoku.relaxation_mechanism" "reclassify")))
  (define (either-class . values)
    (or (and (memv before-class values) #t) (and (memv after-class values) #t)))
  (define (either-key . values)
    (or (and (memv before-key values) #t) (and (memv after-key values) #t)))
  ;; The three §B.2 notes 14 through 16 relax, where the caller asked for the
  ;; boundary to be opened rather than for the class to change.
  (define (line-head-classes)
    (or (memv after-class '(10 11))
        (memv before-class '(10 11))
        (and (not (answer-is? style "kinsoku.iteration_mark_at_line_head" "prohibited"))
             (either-key ideographic-iteration-mark))))
  (case level
    [(1) #t]
    [(2)
     (or (either-class 3)
         (either-key katakana-middle-dot percent-sign)
         (and (= before-class 8)
              (= after-class 8)
              (or (and (eqv? before-key horizontal-ellipsis) (eqv? after-key horizontal-ellipsis))
                  (and (eqv? before-key two-dot-leader) (eqv? after-key two-dot-leader))))
         (and matrix? (line-head-classes) #t))]
    [(3) (and matrix? (line-head-classes) #t)]
    [else #f]))

;; ----------------------------------------------------------------------------
;; The answer
;; ----------------------------------------------------------------------------

;; Whether a line may end between these two occurrences.
(define (breakable? para style before after)
  (define before-class (item-class before))
  (define after-class (item-class after))
  (define before-key (single-key para before))
  (define after-key (single-key para after))
  (define cell (tables:cell-at table2 before-class after-class))
  (define base
    (cond
      ;; §C.2 note 5: the cell is blank, and five ordered couples are still one unit.
      [(and (= before-class 8) (= after-class 8)) (not (indivisible-couple? before-key after-key))]
      ;; §C.2 note 11: a quantity symbol or a European numeral holds its postfix.
      [(and (= before-class 27) (= after-class 13))
       (not (or (eq? (item-role before) 'quantity-symbol) (european-numeral? before-key)))]
      ;; §C.2 note 10: the caller's own answer.
      [(and (= before-class 24) (= after-class 27))
       (answer-is? style "kinsoku.grouped_numeral_before_western" "breakable")]
      [(eq? cell 'blank) #t]
      [else #f]))
  (or base
      (and (not (always-prohibited? before-class after-class))
           (not (structural? before-class))
           (not (structural? after-class))
           (relaxed? style (kinsoku-level style) before after before-class after-class before-key after-key))))
