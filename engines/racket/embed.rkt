#lang racket/base
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; Pasting a file from `spec/` into the compiled module, as a string literal.
;;
;; "The runner contract" of docs/design/conformance.md is what forces this: the
;; suite runner starts an engine with `Command::new(engine)`, no arguments, from a
;; working directory it never discloses, so there is no path for the engine to
;; resolve at run time and every reference table it needs has to be inside the
;; executable.
;;
;; Racket offers two ways to get a data file into a `raco exe` binary and only one
;; of them satisfies that sentence.
;;
;; - `define-runtime-path` records the path and leaves the *file* where it is.
;;   `raco exe` alone does not copy it anywhere; only a subsequent
;;   `raco distribute` gathers runtime-path files next to the executable, and
;;   without that step the binary resolves the absolute path it was built at. That
;;   works on the machine that built it and stops being true the moment the
;;   repository moves, which is exactly the run-time file resolution the contract
;;   rules out.
;; - Reading the file at *expansion* time and expanding to its contents puts the
;;   bytes in the compiled code. `raco make` writes them into the `.zo`, `raco exe`
;;   embeds the `.zo`, and the executable has no file to find. That is what
;;   `embed-file` does, and it is the same shape as the OCaml engine's
;;   `lib/specdata/dune` rule: build-time paste, run-time parse, nothing generated
;;   into the source tree and nothing committed.
;;
;; The one thing a compile-time read costs is that Racket's compilation manager
;; cannot see the dependency by itself -- `raco make` tracks `require`s, not files a
;; macro happened to open -- so a table could change under a stale `.zo`.
;; `register-external-file` from `compiler/cm-accomplice` records the file in the
;; module's `.dep`, which is the supported way to say "recompile me when this
;; changes" and is what makes `just build-engine-racket` correct after a
;; `just derive`.

(require (for-syntax racket/base
                     racket/file
                     racket/path
                     compiler/cm-accomplice))

(provide embed-file)

;; `(embed-file "../../spec/derived/classes.tsv")` expands to that file's contents
;; as an immutable string. The path is relative to the source file that writes the
;; form, which is the same rule `include` and `define-runtime-path` follow.
(define-syntax (embed-file stx)
  (syntax-case stx ()
    [(_ relative)
     (let ([literal (syntax-e #'relative)]
           [source (syntax-source stx)])
       (unless (string? literal)
         (raise-syntax-error 'embed-file "expected a literal path string" stx #'relative))
       (unless (path? source)
         (raise-syntax-error 'embed-file
                             "the enclosing module has no source path to resolve against"
                             stx))
       (let ([absolute (simplify-path (build-path (path-only source) literal) #f)])
         (unless (file-exists? absolute)
           (raise-syntax-error 'embed-file (format "~a does not exist" absolute) stx #'relative))
         ;; Make `raco make` recompile this module when the table changes.
         (register-external-file absolute)
         (datum->syntax stx
                        (string->immutable-string (file->string absolute #:mode 'binary))
                        stx)))]))
