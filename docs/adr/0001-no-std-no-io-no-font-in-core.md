# ADR-0001: the layout core has no std, no I/O, and no font access

- Status: accepted; the historical crate enumeration is superseded by
  [ADR 0022](0022-unified-public-crate-and-process-conformance.md). The purity rule itself
  is unchanged and now binds the single `jlreq` library.
- Date: 2026-08-05

## Context

Every existing implementation of Japanese line composition is fused to a rendering stack.
The `jlreq` LaTeX class is inseparable from TeX, browser implementations from their layout
engines, InDesign's from InDesign. That fusion is the reason none of them is reusable, and
it is the reason the rules have never been extracted as data anyone can test against.

Repeating that mistake is easy. Line composition needs to know how wide a character is,
and the shortest route to that is to open a font. Once a layout crate loads fonts it needs
a filesystem, then a font database, then a cache, then a threading story — and it can no
longer be used from a browser, a game engine with its own asset pipeline, a PDF writer
that already has its fonts, or a test that wants a fixed answer.

## Decision

The crate names below record the pre-1.0 topology in which this decision was made. In the
current topology, their code is private inside `jlreq`, and the same rule applies to the
whole library.

`jlreq-unit`, `jlreq-spec`, `jlreq-class`, `jlreq-spacing`, `jlreq-line`, `jlreq-inline`,
and `jlreq` are `no_std`. They do not depend on `std`, on any I/O crate, on any font
crate, or on any allocator beyond `alloc` where composition genuinely needs to build a
result.

`jlreq-unit`, `jlreq-spec`, `jlreq-class`, and `jlreq-spacing` do not allocate at all.

The gate is mechanical, not cultural. `just purity` reads every core manifest and source
file and fails on a `std` dependency, an I/O or font dependency, or a `std::` path.
`just no-std` builds the core for `thumbv7em-none-eabi`, and `just wasm` builds it for
`wasm32-unknown-unknown`. All three run in CI and in the pre-commit hook.

## Consequences

The library cannot answer "how wide is this character" on its own; the caller must supply
it ([ADR 0002](0002-caller-supplied-metrics.md)). That is a real constraint on the API and
it is accepted deliberately.

In exchange: the conformance suite runs with no font files and no fixtures beyond text and
numbers, the crate is usable in `no_std` and WebAssembly contexts that no font-coupled
layout engine can reach, and the dependency surface a security-conscious adopter has to
review is empty.

The purity gate is written before any layout code exists, so the first violation is caught
on the first commit that would introduce it rather than during a later cleanup that never
happens.
