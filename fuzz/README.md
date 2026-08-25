<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Fuzzing

The fuzz suite separates the three failure domains that matter at the public boundary:

- `input_validation` feeds malformed and valid caller-owned byte ranges, advances,
  breaks, tabs, and all inline structures into the validated paragraph model.
- `composition` exercises accepted paragraphs, every style and writing mode, arithmetic
  boundaries, resource limits, and all result views.
- `protocol_parser` feeds arbitrary and malformed NDJSON through the conformance protocol
  parser and then validates any request that was successfully decoded.

Run all three bounded CI workloads with `just fuzz-check`, or continue one target locally
with:

```console
cargo +nightly fuzz run composition --fuzz-dir fuzz
```

Reviewed, coverage-minimized inputs live in `fuzz/seeds/<target>/`. The recipes copy them
to `target/fuzz-corpus/` before execution, so libFuzzer's evolving corpus never changes the
working tree. Existing files under the legacy `fuzz/corpus/public_api/` are retained as
source material only; minimize a useful input and move the result into the matching seed
directory before committing it.
