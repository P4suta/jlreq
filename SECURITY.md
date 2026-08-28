# Security Policy

## Scope and threat model

`jlreq` is intended for servers, editors, and document pipelines that may receive
attacker-controlled text and font bytes. Both inputs are untrusted. A panic, hang,
out-of-bounds access, integer overflow, unbounded allocation, or partial result presented as
complete is a security defect.

The facade parses TTF/OTF/TTC data, itemizes arbitrary Unicode, resolves bidi, shapes runs,
segments lines, and lowers typed structures. The dependency-free core consumes the
validated pre-shaped representation and performs exact composition. The conformance runner
also launches an explicitly supplied engine process and exchanges bounded NDJSON.

Specifically in scope:

- malformed font directories, tables, offsets, TTC indices, variation data, and glyph data;
- arbitrary valid UTF-8, including empty input, controls, combining marks, variation
  selectors, deeply alternating bidi text, long unbreakable spans, and many paragraphs;
- invalid UTF-8 byte ranges or crossing typed document structures supplied through the
  Rust API;
- NaN, infinity, negative or out-of-range geometry, and arithmetic overflow after
  quantization;
- resource exhaustion through input bytes, font bytes/count, paragraphs, shaping runs,
  glyphs, constructs, break candidates, tabs, or exact-search transitions;
- protocol messages, suites, stderr, child-process lifetime, and response cardinality.

Rasterizers and renderers are outside this repository. A visual preference where JLReq
permits alternatives is a conformance or design issue, not a vulnerability.

## Bounds and atomic failure

`LayoutOptions::limits` accepts `ResourceLimits` for:

| Resource | Default maximum |
| --- | ---: |
| input UTF-8 bytes | 16 MiB |
| registered font faces | 256 |
| total font bytes | 512 MiB |
| paragraphs | 65,536 |
| shaping runs | 262,144 |
| produced glyphs | 1,000,000 |
| typed constructs | 4,096 |
| charged core operations | 8,000,000 |

The core separately bounds clusters and break candidates at 65,536 each, constructs and tab
stops at 4,096 each, and exact-search transitions at 8,000,000 by default. Callers should
lower these limits for latency-sensitive services.

All public floating-point values are validated and immediately quantized to signed 26.6
fixed point. Invalid fonts, invalid options, invalid documents, and exceeded limits return a
typed `LayoutError` and no `TextLayout`. Core composition likewise returns no partial
`Layout`. Reusable engines retain only valid immutable caches and scratch capacity, so an
error cannot poison the next request.

Fallback never drops input. When no face covers a grapheme, the primary `.notdef`, original
source range, and `font.missing-glyph` diagnostic remain in the complete result.

The conformance runner defaults to 1 MiB per message, 256 MiB per suite, 200,000 cases, a
30-second inactivity timeout, bounded retained stderr, concurrent pipe draining, watchdog
termination, and child cleanup.

## Assurance

Tests cover valid fixture fonts, malformed font bytes, arbitrary text/options, resource
limits, failure recovery, and deterministic one-shot/reused results. Fuzz targets exercise
both valid-font arbitrary-text layout and malformed font registration/layout. Coverage,
mutation testing, CodeQL, `cargo deny`, REUSE, package isolation, and SHA-pinned workflow
audits are release gates.

## Reporting

Report privately through GitHub's [Report a vulnerability][advisories] flow, not a public
issue. Include the smallest reproducing text/font if disclosure is safe, the options and
limits used, target and Rust versions, and observed behavior. Expect acknowledgement within
seven days.

## Supported versions

The prepared 0.1.x line receives security fixes. Before first publication, reports against
the prepared 0.1.0 tree and `main` follow the same policy.

| Version | Supported |
| --- | --- |
| `0.1.x` | Supported |
| earlier development snapshots | Unsupported |

[advisories]: https://github.com/P4suta/jlreq/security/advisories/new
