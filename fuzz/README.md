<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Public API fuzzing

The `public_api` target feeds malformed and valid caller-owned byte ranges, advances,
breaks, tabs, and all nine inline structures through the same public API an integration
uses. Rejection by `ShapedText`, `Ruby`, or `ParagraphBuilder` is expected; every accepted
paragraph must compose and expose all result views without panicking.

Run the bounded CI workload with `just fuzz-check`, or continue exploring locally with:

```console
cargo +nightly fuzz run public_api
```

The committed corpus keeps invalid UTF-8 boundaries, overlapping ranges, Appendix A pair
splits, extreme arithmetic, crossing constructs, and construct-internal breaks in every
regression run. Crashes minimized by cargo-fuzz belong in `corpus/public_api/`; generated
artifacts remain ignored.
