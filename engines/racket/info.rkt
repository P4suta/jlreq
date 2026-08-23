#lang info
;; SPDX-FileCopyrightText: 2026 jlreq contributors
;; SPDX-License-Identifier: MIT OR Apache-2.0

;; This directory is not a published package and is never installed with
;; `raco pkg`. The file exists to state, in the form Racket's own tooling reads,
;; exactly which collections the engine and its tests need -- which is what the
;; `conform-racket` job installs on top of a `distribution: minimal` Racket, and
;; the only list either place has to agree with.
;;
;; The engine itself depends on `base` and nothing else: `json`, the reader, the
;; string and list libraries and the macro that pastes `spec/` into the compiled
;; module are all in it. An independent reference implementation that pulled in a
;; JSON package would be testing that package's agreement with `serde_json` rather
;; than this project's agreement with JLReq, which is the same reason the Rust core
;; carries no dependencies and the OCaml engine declares only the compiler and
;; dune.
;;
;; `compiler-lib` is a build dependency and not a run-time one: it is what supplies
;; `raco make`, `raco exe` and `raco test`, none of which minimal Racket ships.
;; `rackunit-lib` is the test framework.

(define collection "jlreq-engine-racket")
(define pkg-desc "An independent Racket implementation of the jlreq conformance protocol")
(define pkg-authors '("jlreq contributors"))
(define license '(MIT OR Apache-2.0))
(define version "0.0.0")

(define deps '("base"))
(define build-deps '("compiler-lib" "rackunit-lib"))
