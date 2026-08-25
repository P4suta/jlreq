<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Changelog

All notable user-facing changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Detailed pre-release development
chronology is archived in [DEVELOPMENT-HISTORY.md](DEVELOPMENT-HISTORY.md).

## [Unreleased]

These are the completed 0.1.0 release notes. Publication will replace this heading with the
approved release date; no crate upload, tag, or GitHub Release has occurred yet.

### Added

- A dependency-free `no_std + alloc` `jlreq` library for exact integer Japanese paragraph
  composition over caller-shaped UTF-8 clusters.
- Horizontal and vertical placement, nine inline construct types, mono/group/jukugo ruby,
  tabs, widow control, diagnostics, and 22 typed JLReq 2020 style choices.
- Deterministic `CompositionLimits`, reusable `Composer`, and typed `ComposeError` with
  stable code, resource, limit, and observed-count fields.
- The binary-only `jlreq-conformance` protocol-v1 runner, built-in suite, JSON Schema, and
  sample engine, with streaming transport, bounded input/output, inactivity timeout, and
  order-independent response matching by unique case ID.
- Independent OCaml and Racket engines plus ten generated three-way censuses containing
  122,199 cases with zero expected differences.
- Reproducible specification generation and attestation, API/error-code controls, coverage,
  mutation, fuzzing, package, MSRV, `no_std`, WASM, and release-artifact gates.

### Changed

- The final package and binary names are `jlreq`, `jlreq-conformance`, and
  `jlreq-sample-engine`; older experimental multi-crate names are not supported.
- `compose` and `Composer::compose` return `Result<Layout, ComposeError>`. Callers must
  handle deterministic resource refusal; successful values are always complete exact
  layouts. There is no infallible compatibility wrapper.

### Security

- Exact search is charged against a transition budget; parser/message/suite/case limits,
  concurrent stderr draining, bounded stderr retention, watchdog termination, and process
  cleanup prevent untrusted input or engines from causing unbounded work or pipe deadlock.

[Unreleased]: https://github.com/P4suta/jlreq/compare/v0.1.0...HEAD
