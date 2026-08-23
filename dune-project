(lang dune 3.16)

; SPDX-FileCopyrightText: 2026 jlreq contributors
;
; SPDX-License-Identifier: MIT OR Apache-2.0
;
; `(lang dune ...)` must be the first line of this file, so the license header
; follows it rather than leading it. Every other new file in this tree leads with
; the header.

; The dune project exists only for `engines/`. The Rust workspace is unaffected:
; `dirs` in the sibling `dune` keeps dune from scanning `crates/`, `xtask/`, or
; `target/`, and no Cargo member ever depends on anything below.
;
; The root is here rather than under `engines/ocaml/` because a dune rule cannot
; depend on a file above its project root, and the engines read `spec/*.tsv`
; directly (ADR 0009: the transcribed and derived tables are the source, the
; generated Rust is not).

(name jlreq)
