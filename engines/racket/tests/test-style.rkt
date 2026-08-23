#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The twenty-two questions, resolved.
;;
;; `spec/derived/questions.tsv` is the whole of what this module knows, so what is
;; worth testing is the resolution rather than the answers: which profile a request
;; that names none falls back to, that a stated setting outranks the profile, that a
;; value outside a question's own vocabulary is refused, and that the one pair of
;; answers the specification excludes is refused whether the second half was stated
;; or came from the profile.

(require rackunit racket/list "../style.rkt")

(module+ test
  ;; Every question of the file is present, and each profile answers all of them.
  (check-equal? (length style-questions) 22)

  ;; A request that states no style at all takes the profile the document's own
  ;; preferences add up to.
  (define none (resolve-style 'jlreq-default))
  (check-equal? (answer none "kinsoku.level") "strict")
  (check-equal? (answer none "adjustment.reduction_table") "table-3")
  (check-equal? (answer none "adjustment.remainder") "leading")

  ;; A profile by name, and the same profile named inside an object.
  (check-equal? (answer (resolve-style "book-2020") "adjustment.hanging_punctuation") "hanging")
  (check-equal? (answer (resolve-style "newspaper-2020") "kinsoku.level") "very-loose")
  (check-equal? (answer (resolve-style "magazine-2020") "kinsoku.level") "loose")
  (check-equal? (answer (resolve-style "jis-reading-2020") "adjustment.reduction_table") "table-4")
  (check-equal? (answer (resolve-style (hasheq 'profile "book-2020")) "spacing.line_head_opening_bracket")
                "pattern-3")

  ;; A stated setting outranks the profile, and leaves the rest of it alone.
  (define mixed
    (resolve-style (hasheq 'profile "book-2020" 'adjustment.hanging_punctuation "none")))
  (check-equal? (answer mixed "adjustment.hanging_punctuation") "none")
  (check-equal? (answer mixed "spacing.line_head_opening_bracket") "pattern-3")
  (check-true (answer-is? mixed "kinsoku.level" "strict"))

  ;; An object with no profile is the default profile with the stated answers on top.
  (check-equal? (answer (resolve-style (hasheq 'kinsoku.level "very-strict"
                                               'kinsoku.grouped_numeral_before_western "unbreakable"
                                               'kinsoku.relaxation_mechanism "matrix"))
                        "adjustment.reduction_table")
                "table-3")

  ;; ------------------------------------------------------------------
  ;; What is refused
  ;; ------------------------------------------------------------------

  (check-exn exn:fail? (lambda () (resolve-style "book-2021")) "a profile that does not exist")
  (check-exn exn:fail?
             (lambda () (resolve-style (hasheq 'kinsoku.level "quite-strict")))
             "an answer the question does not permit")
  (check-exn exn:fail?
             (lambda () (resolve-style (hasheq 'kinsoku.tolerance "3")))
             "a question the specification does not state")
  (check-exn exn:fail? (lambda () (resolve-style 7)) "a style that is neither a name nor an object")

  ;; §C.3's fourth level excludes what `jlreq-2020` answers to two other questions,
  ;; so the level alone is a contradiction: the profile's answer is as much a part
  ;; of the resolved style as a stated one.
  (check-exn exn:fail?
             (lambda () (resolve-style (hasheq 'kinsoku.level "very-strict")))
             "very-strict against the profile's own breakable grouped numeral")
  (check-exn exn:fail?
             (lambda ()
               (resolve-style (hasheq 'kinsoku.level "very-strict"
                                      'kinsoku.grouped_numeral_before_western "unbreakable")))
             "one half answered and the other still the profile's")
  ;; Both halves answered, and it resolves.
  (check-equal? (answer (resolve-style (hasheq 'kinsoku.level "very-strict"
                                               'kinsoku.grouped_numeral_before_western "unbreakable"
                                               'kinsoku.relaxation_mechanism "matrix"))
                        "kinsoku.level")
                "very-strict")

  ;; `answer` is closed over the same list.
  (check-exn exn:fail? (lambda () (answer none "kinsoku.tolerance"))))
