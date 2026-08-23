#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; The twenty-two places JLReq permits more than one answer.
;;
;; `spec/derived/questions.tsv` is the whole of this module's knowledge: which
;; questions exist, what each permits, what each of the five dated profiles answers,
;; and which pairs of answers the specification refuses to hold at once. Nothing
;; below hard-codes a question name or an answer, so a revision that adds a question
;; arrives here by regenerating that file.
;;
;; Resolution
;;
;; A request states either a profile name or an object; the object may name a
;; profile and may state any of the dotted settings beside it. An unstated question
;; takes the profile's answer, and a request that states no style at all takes
;; `jlreq-2020`, which is the profile the document's own preferences add up to.
;;
;; Exclusions
;;
;; The `excludes` column carries the pairs that cannot both hold. There is one
;; today: §C.3's `very-strict` level excludes both
;; `kinsoku.grouped_numeral_before_western: breakable` and
;; `kinsoku.relaxation_mechanism: reclassify`, which are what `jlreq-2020` answers.
;; A request that reaches such a pair is refused rather than answered: the two
;; settings describe different paragraphs and picking one of them silently would
;; publish a layout the caller did not ask for. It is refused whether the second
;; answer was stated or came from the profile, because the profile's answer is as
;; much a part of the resolved style as a stated one.

(require racket/string (prefix-in tables: "tables.rkt") "tsv.rkt" "model.rkt")

(provide resolve-style
         answer
         answer-is?
         style-questions)

;; ----------------------------------------------------------------------------
;; The question table
;; ----------------------------------------------------------------------------

;; One question: what it permits, what each profile answers, what it excludes.
(struct question (name permits profiles excludes) #:transparent)

;; The profile column of `questions.tsv` each protocol name reads.
(define profile-columns
  '(("jlreq-2020" . "jlreq")
    ("jis-reading-2020" . "jis_reading")
    ("book-2020" . "book")
    ("magazine-2020" . "magazine")
    ("newspaper-2020" . "newspaper")))

(define (words text separator)
  (filter (lambda (piece) (not (string=? piece ""))) (string-split text separator #:trim? #t)))

(define style-questions
  (let* ([file tables:questions]
         [name-column (tsv-column file "question")]
         [permits-column (tsv-column file "permits")]
         [excludes-column (tsv-column file "excludes")])
    (for/list ([row (in-list (tsv-rows file))])
      (question (string-trim (tsv-field row name-column))
                (words (tsv-field row permits-column) " ")
                (for/list ([pair (in-list profile-columns)])
                  (cons (car pair) (string-trim (tsv-field-of file row (cdr pair)))))
                (for/list ([rule (in-list (words (tsv-field row excludes-column) ";"))])
                  (define pieces (string-split rule "|" #:trim? #f))
                  (unless (>= (length pieces) 3)
                    (fail-input "the Style questions state the exclusion `~a`" rule))
                  (list (car pieces) (cadr pieces) (caddr pieces)))))))

(define question-index
  (let ([table (make-hash)])
    (for ([one (in-list style-questions)])
      (hash-set! table (question-name one) one))
    table))

(define (question-named name)
  (or (hash-ref question-index name #f)
      (fail-input "`~a` is not one of the Style questions" name)))

;; ----------------------------------------------------------------------------
;; Reading the request's own answer
;; ----------------------------------------------------------------------------

;; The profile every unstated question falls back to when the request names none.
(define default-profile "jlreq-2020")

;; A resolved style is a hash from question name to answer, both strings.
(define (resolve-style stated)
  (define-values (profile settings)
    (cond
      [(eq? stated 'jlreq-default) (values default-profile '())]
      [(string? stated)
       (unless (assoc stated profile-columns)
         (fail-input "`~a` is not one of the Style profiles" stated))
       (values stated '())]
      [(hash? stated)
       (define named (hash-ref stated 'profile #f))
       (when (and named (not (string? named)))
         (fail-input "the style names a profile that is not a string"))
       (when (and named (not (assoc named profile-columns)))
         (fail-input "`~a` is not one of the Style profiles" named))
       (values (or named default-profile)
               (for/list ([(key value) (in-hash stated)] #:unless (eq? key 'profile))
                 (cons (symbol->string key) value)))]
      [else (fail-input "style is neither a profile name nor an object")]))
  (define answers (make-hash))
  (for ([one (in-list style-questions)])
    (hash-set! answers (question-name one) (cdr (assoc profile (question-profiles one)))))
  (for ([pair (in-list settings)])
    (define one (question-named (car pair)))
    (define value (cdr pair))
    (unless (string? value)
      (fail-input "the style answers `~a` with something that is not a string" (car pair)))
    (unless (member value (question-permits one))
      (fail-input "the style answers `~a` with `~a`, which the specification does not permit"
                  (car pair)
                  value))
    (hash-set! answers (car pair) value))
  (check-exclusions answers)
  answers)

;; Refuse a resolved style that holds both halves of an excluded pair.
(define (check-exclusions answers)
  (for ([one (in-list style-questions)])
    (define chosen (hash-ref answers (question-name one)))
    (for ([rule (in-list (question-excludes one))])
      (define when-answer (car rule))
      (define other (cadr rule))
      (define forbidden (caddr rule))
      (when (and (string=? chosen when-answer)
                 (string=? (hash-ref answers other) forbidden))
        (fail-input "`~a: ~a` excludes `~a: ~a`, and this style holds both"
                    (question-name one)
                    when-answer
                    other
                    forbidden)))))

;; The answer to one question.
(define (answer style name)
  (hash-ref style name (lambda () (fail-input "`~a` is not one of the Style questions" name))))

(define (answer-is? style name value)
  (string=? (answer style name) value))
