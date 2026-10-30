<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Documentation

Version 0.1.0 remains unreleased.
Use the source checkout until publication, and consult the [release procedure](RELEASING.md) for the candidate state.
The `jlreq` library requires Rust 1.88.
The core and conformance runner require Rust 1.85.

The [English guide](guide.md) introduces the current API with executable examples.
The [crate guide](../crates/jlreq/README.md) describes fonts, shaping, composition, and the rendering contract.
The [Japanese guide](guide.ja.md) covers usage and editing.
The [coordinate contract](design/geometry.md) explains layout cells and drawing baselines.
Fonts, rasterization, and drawing remain the caller's responsibility.

Known layout defects remain under investigation, including the [captured geometry counterexample](https://github.com/P4suta/jlreq/issues/45).
The project doesn't claim complete conformance to [Japanese Industrial Standards (JIS)](https://www.jisc.go.jp/qa/a3-1.html) X 4051.
Read the [conformance design](design/conformance.md) and [API inventory](public-api.toml) for the declared scope.

See the [architecture](../ARCHITECTURE.md), [security policy](../SECURITY.md), and [contribution guide](../CONTRIBUTING.md) for development.
