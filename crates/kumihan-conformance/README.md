<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# kumihan-conformance

`kumihan-conformance` is the binary-only, language-independent black-box conformance runner
for JLReq 2020 line-composition engines. It speaks NDJSON with an external engine process;
every message identifies `kumihan.conformance/1` and
`jlreq-2020-08-11+unicode-17.0.0`.

```text
kumihan-conformance list [SUITE.ndjson]
kumihan-conformance validate [SUITE.ndjson|-]
kumihan-conformance run ENGINE [SUITE.ndjson]
```

Exit status 0 means conformance or valid input, 1 means an observable mismatch, and 2 means
an input, protocol, or engine error. The package includes `protocol.schema.json`, the
built-in suite, and `kumihan-sample-engine`; it intentionally has no library target.

See the [protocol design](https://github.com/P4suta/kumihan/blob/main/docs/design/conformance.md)
and [main repository guide](https://github.com/P4suta/kumihan) for the unreleased candidate
contract.
