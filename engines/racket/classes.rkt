#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; §3.9.2: which of the thirty classes an occurrence belongs to.
;;
;; Appendix A enumerates 1133 keys and names 473 of them under more than one class,
;; so a lookup is not a function of a code point. It is a function of an
;; *occurrence*: the key, the frame the caller set it on, the writing mode, and the
;; role the caller declared. That is the axis set JLReq itself works from — §3.2.4
;; reads a Western character set full-width or fixed-width as quasi-Japanese, §3.2.6
;; reads the same character set proportionally as Western, and Appendix A's Remarks
;; column qualifies a listing by the very same facts.
;;
;; The order below is the order the specification stops being silent in.
;;
;; 1. Folding. Appendix A keys the narrow form and real text carries the wide one,
;;    so `spec/derived/folding.tsv`'s Wide and Narrow decompositions — and no other
;;    compatibility mapping, ADR 0008 — are applied first.
;;
;; 2. The declared role. `grouped-numeral`, `decimal-point` and
;;    `digit-group-separator` name §A.24's class, `unit-symbol` names §A.25's, and
;;    `warichu-bracket` names §A.28's or §A.29's. A role is the caller saying which
;;    construct the occurrence is inside, which is the one axis `classify` cannot
;;    read off the character, so a role that the key has a listing for settles the
;;    question outright.
;;
;; 3. Qualification. A Remarks cell states the width the listing is for
;;    (`proportionally-spaced`, `half-width`, `quarter em width`), the writing mode
;;    it is used in, or nothing. A listing whose cell names a width the protocol's
;;    `frame` vocabulary can express is available on that frame alone; one whose
;;    cell names only a width no caller can declare — a quarter em, a third of an
;;    em — is available on none, which is the reading `engines/ocaml/README.md`
;;    records for U+0020 and U+2010.
;;
;;    Two classes carry a width JLReq states in its own prose rather than in a
;;    Remarks cell. §3.1.2 gives cl-01, cl-02, cl-05, cl-06 and cl-07 a half-width
;;    character advance, and §3.9.2's own note on cl-27 says the Japanese and the
;;    Western design of the same parenthesis differ in width; §3.2.4 and §3.2.6
;;    divide Western text the same way, cl-19 when it is full-width or fixed-width
;;    and cl-27 when it is proportional. So those six classes are not available on
;;    the proportional frame: a proportional occurrence of one of their keys is the
;;    Western design of it.
;;
;;    All of this is a filter that can empty the candidate set, and an empty set is
;;    not an answer. Where nothing survives, every listing stands again: a key
;;    Appendix A lists only at a width the caller cannot declare is still that key,
;;    and answering `Unlisted` for U+2010 because the protocol has no word for a
;;    quarter em would be reporting a fact about the protocol as a fact about the
;;    character.
;;
;; 4. The tie-break. `docs/decisions/ambiguous-context.md`: the lowest-numbered
;;    survivor that is not membership in a construct the caller never declared, and
;;    the lowest-numbered survivor of all where every survivor is one.
;;    `classification.ambiguous_context: highest-class` takes the highest instead.
;;
;; 5. Nothing listed. `docs/decisions/unlisted-code-point.md`: cl-27 on the
;;    proportional frame and cl-19 otherwise, or cl-19 always under
;;    `classification.unlisted_code_point: ideographic`.

(require racket/list
         (prefix-in tables: "tables.rkt")
         "model.rkt"
         "style.rkt")

(provide classify
         classify-cluster
         math-class-of
         construct-class?
         middle-dot-in-construct?
         folded-key
         script-of
         unified-ideograph?
         line-edge)

;; The class number Tables 1 and 3 through 5 address the line edge by.
(define line-edge 0)

;; The nine classes that are membership *in* a construct: the five that enumerate
;; no code point at all and the four that enumerate what may appear inside a
;; grouped numeral, a unit symbol or a warichu bracket. `classify` is given no
;; construct axis beyond the caller's own `role`, so as far as it can know an
;; occurrence that declares none is inside none.
(define (construct-class? value)
  (or (<= 20 value 25) (= value 28) (= value 29) (= value 30)))

;; ----------------------------------------------------------------------------
;; The tables, indexed
;; ----------------------------------------------------------------------------

;; Every listing of a key, in Appendix A's own document order.
(define listings-by-key
  (let ([table (make-hash)])
    (for ([row (in-list (reverse tables:appendix-a))])
      (hash-update! table
                    (tables:listing-key row)
                    (lambda (rows) (cons (cons (tables:listing-class row) (tables:listing-remark-en row)) rows))
                    '()))
    table))

;; The Wide and Narrow compatibility decompositions, source to target.
(define folding-map
  (let ([table (make-hash)])
    (for ([one (in-list tables:folding)])
      (hash-set! table (tables:fold-source one) (tables:fold-target one)))
    table))

;; Whether the scalar is `Unified_Ideograph`, which is the whole of the cl-19
;; membership §A.19's own table deliberately does not enumerate.
(define (unified-ideograph? scalar)
  (for/or ([range (in-list tables:ideographs)])
    (<= (tables:range-entry-first range) scalar (tables:range-entry-last range))))

;; `Hiragana`, `Katakana` or #f, which §C.2 note 3's small-kana fallback reads.
(define (script-of scalar)
  (for/or ([range (in-list tables:scripts)])
    (and (<= (tables:script-range-first range) scalar (tables:script-range-last range))
         (tables:script-range-script range))))

;; ----------------------------------------------------------------------------
;; Reading a Remarks cell
;; ----------------------------------------------------------------------------

(define (contains? text piece)
  (define limit (- (string-length text) (string-length piece)))
  (let search ([index 0])
    (cond
      [(> index limit) #f]
      [(string=? (substring text index (+ index (string-length piece))) piece) #t]
      [else (search (add1 index))])))

;; The frames a Remarks cell names, or #f where it names no width at all.
;;
;; The empty list is a cell that names widths and none of them expressible: a
;; quarter em, a third of an em. That is not the same answer as #f, and the
;; distinction is the whole of the U+0020 / U+2010 reading.
(define (remark-frames remark)
  (define wide? (contains? remark "proportionally-spaced"))
  (define half? (contains? remark "half-width"))
  (define quarter? (contains? remark "quarter em width"))
  (define third? (contains? remark "one third em width"))
  (cond
    [(or wide? half? quarter? third?)
     (append (if half? '(half-em) '()) (if wide? '(proportional) '()))]
    [else #f]))

;; The writing mode a Remarks cell names, or #f.
(define (remark-writing-mode remark)
  (cond
    [(contains? remark "used in horizontal composition") 'horizontal-tb]
    [(contains? remark "used in vertical composition") 'vertical-rl]
    [else #f]))

;; The six classes JLReq gives a Japanese character advance to in its own prose.
;; A proportional occurrence of one of their keys is the Western design of it.
(define (japanese-design? value)
  (or (= value 1) (= value 2) (= value 5) (= value 6) (= value 7) (= value 19)))

;; Whether one listing is used in this writing mode.
(define (mode-fits? remark writing-mode)
  (define mode (remark-writing-mode remark))
  (or (not mode) (eq? mode writing-mode)))

;; Whether one listing is set at this width.
(define (width-fits? value remark frame)
  (define frames (remark-frames remark))
  (and (if frames (and (memq frame frames) #t) #t)
       (not (and (eq? frame 'proportional) (japanese-design? value)))))

;; Whether one listing describes this occurrence.
(define (listing-available? value remark frame writing-mode)
  (and (mode-fits? remark writing-mode) (width-fits? value remark frame)))

;; ----------------------------------------------------------------------------
;; Classification
;; ----------------------------------------------------------------------------

;; The class a declared role names, or #f.
(define (role-class role)
  (case role
    [(grouped-numeral decimal-point digit-group-separator) 24]
    [(unit-symbol) 25]
    [else #f]))

;; The classes `key` is listed under, with the Remarks cell of each.
(define (listings-of key)
  (define stated (hash-ref listings-by-key key '()))
  (cond
    [(and (= (length key) 1) (unified-ideograph? (car key)) (not (assv 19 stated)))
     (append stated (list (cons 19 "")))]
    [else stated]))

;; The class of one occurrence.
;;
;; `key` is the folded scalar sequence, `frame` and `writing-mode` are the
;; occurrence's own, `role` is what the caller declared, and `style` answers
;; §3.9.2's three open questions.
(define (classify key frame writing-mode role style)
  (define stated (listings-of key))
  (cond
    [(null? stated) (unlisted-class frame style)]
    [else
     (define forced (role-class role))
     ;; §3.4.2 and §3.9.2: the brackets that close an inline cutting note are cl-28
     ;; and cl-29, which are their own classes precisely because they stand where the
     ;; line does and are set against it differently from ordinary brackets. §A.28 and
     ;; §A.29 enumerate them, and the caller's `warichu-bracket` role is what
     ;; disambiguates a key those sections list from the same key in §A.01 or §A.02 --
     ;; so a note bracketed with a character neither section lists is a note bracketed
     ;; with an ordinary bracket and not a fourth member of those classes.
     (define warichu
       (and (eq? role 'warichu-bracket)
            (cond
              [(assv 28 stated) 28]
              [(assv 29 stated) 29]
              [else #f])))
     (define available
       (for/list ([one (in-list stated)]
                  #:when (listing-available? (car one) (cdr one) frame writing-mode))
         one))
     ;; Two fallbacks, because the conditions a Remarks cell states are not all the
     ;; same kind of claim.
     ;;
     ;; A cell that says `proportionally-spaced` names the Western design of the
     ;; character, and §3.9.2's own note on cl-27 is what makes that a different
     ;; character rather than the same one measured differently -- so it stays out
     ;; wherever the caller did not set the occurrence proportionally. Every other
     ;; width is a measurement, and where no listing was taken at the caller's own
     ;; the width stops separating them. The writing mode outlasts both: a full stop
     ;; §A.06 lists "used in horizontal composition" is not the full stop of a
     ;; vertical line whatever it is measured at, so a vertical U+002E is §A.24's
     ;; decimal point and not §A.06's.
     ;;
     ;; Only where nothing survives even that does every listing stand again, which
     ;; is what answers a key Appendix A lists in one writing mode alone when it is
     ;; read in the other.
     (define nearly
       (for/list ([one (in-list stated)]
                  #:when (and (mode-fits? (cdr one) writing-mode)
                              (or (eq? frame 'proportional)
                                  (not (equal? (remark-frames (cdr one)) '(proportional))))))
         one))
     (define pool
       (cond
         [(not (null? available)) available]
         [(not (null? nearly)) nearly]
         [else stated]))
     (cond
       ;; A role the key has a listing for settles it: the caller has named the
       ;; construct, which is the axis the character itself cannot carry.
       [(and warichu (assv warichu available)) warichu]
       [(and forced (assv forced available)) forced]
       ;; `by-role` reads §A.24's cells as naming a job rather than a width, so an
       ;; occurrence the width would have admitted and the caller declared no job
       ;; for is not a grouped numeral. What is left describing it is Western text
       ;; set at that width, which §A.27 is the listing of.
       [(and (answer-is? style "classification.grouped_numeral_qualification" "by-role")
             (not forced)
             (assv 24 available))
        27]
       [else (tie-break pool style)])]))

;; §3.9.2 concedes the ambiguous case rather than deciding it, and
;; `docs/decisions/ambiguous-context.md` is what answers it.
(define (tie-break pool style)
  (define classes (sort (remove-duplicates (map car pool)) <))
  (define reachable (filter (lambda (value) (not (construct-class? value))) classes))
  (define chosen (if (null? reachable) classes reachable))
  (if (answer-is? style "classification.ambiguous_context" "highest-class")
      (last chosen)
      (first chosen)))

;; A key Appendix A lists nowhere.
(define (unlisted-class frame style)
  (cond
    [(answer-is? style "classification.unlisted_code_point" "ideographic") 19]
    [(eq? frame 'proportional) 27]
    [else 19]))

;; ----------------------------------------------------------------------------
;; From a cluster
;; ----------------------------------------------------------------------------

;; `text` as the scalar sequence Appendix A is looked up by.
;;
;; Appendix A's own preamble requires the Wide and Narrow compatibility
;; decompositions to be folded, because real text carries U+FF08 where §A.1 keys
;; U+0028. It does not require them to be folded over a key the appendix already
;; lists: U+3000 IDEOGRAPHIC SPACE is §A.14's whole enumeration and is also the Wide
;; decomposition source of U+0020, so folding it unconditionally would hand cl-14's
;; one member to cl-26. The fold is therefore what a key falls back to, not what it
;; is looked up as -- which is also what makes it a repair of a rendering difference
;; rather than a normalization of the text.
(define (folded-key text)
  (define raw
    (for/list ([character (in-string text)]) (char->integer character)))
  (if (hash-has-key? listings-by-key raw)
      raw
      (for/list ([scalar (in-list raw)]) (hash-ref folding-map scalar scalar))))

;; The class of one cluster of the paragraph.
;; §3.7.4: whether the key is one of the two math classes, and which.
;;
;; cl-17 and cl-18 are the two classes §3.9.2 closes the set with that no matrix
;; carries an axis for, and a key Appendix A lists under one of them is a math
;; symbol wherever the caller has said the text is a formula. Outside a formula the
;; same key is read like any other: `=` on the proportional frame is Western text.
(define (math-class-of key)
  (for/or ([one (in-list (listings-of key))])
    (and (memv (car one) '(17 18)) (car one))))

(define (classify-cluster para one style)
  (classify (folded-key (cluster-text para one))
            (cluster-frame-of para one)
            (paragraph-writing-mode para)
            (cluster-role one)
            style))

;; §3.1.3's note: a KATAKANA MIDDLE DOT standing inside a unit symbol, a grouped
;; numeral or a formula carries no spacing of its own on either side.
;;
;; The English and the Japanese renderings of that note name different writing
;; modes — the English "in vertical writing mode", the Japanese 横組 — so the
;; reading taken is the union, and the note applies in both. §B.2 note 12 states
;; the same thing about the unit symbol case without naming a mode at all.
(define (middle-dot-in-construct? value role)
  (and (= value 5) (memq role '(grouped-numeral unit-symbol formula)) #t))
