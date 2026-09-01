<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# jlreq-conformance

`jlreq-conformance` is the binary-only, language-independent black-box conformance runner
for JLReq 2020 line-composition engines. It speaks NDJSON with an external engine process;
every message identifies `jlreq.conformance/1` and
`jlreq-2020-08-11+unicode-17.0.0`.

Install with `cargo install jlreq-conformance`.

```text
jlreq-conformance [OPTIONS] list [SUITE.ndjson]
jlreq-conformance [OPTIONS] validate [SUITE.ndjson|-]
jlreq-conformance [OPTIONS] run ENGINE [SUITE.ndjson]
```

Exit status 0 means conformance or valid input, 1 means an observable mismatch, and 2 means
an input, protocol, or engine error. The runner streams requests and responses concurrently,
matches responses by unique `id` in any order, enforces bounded messages/suites/case counts,
and kills an engine after a configurable period without communication. Run `--help` for
`--verbose`, timeout, and size-limit controls.

The package includes `protocol.schema.json`, the built-in suite, and
`jlreq-sample-engine`; it intentionally has no library target.

See the [protocol design](https://github.com/P4suta/jlreq/blob/main/docs/design/conformance.md)
and [main repository guide](https://github.com/P4suta/jlreq) for the 0.1.0 contract.
