# Language-independent conformance protocol

`kumihan-conformance` is a binary-only product. It runs a suite against an external engine
process through NDJSON; an engine in Rust, C++, JavaScript, Python, or any other language
sees the same black-box contract.

## Versioning

Every request and response envelope must contain exactly the protocol and specification
line it implements:

```json
{
  "protocol": "kumihan.conformance/1",
  "spec": "jlreq-2020-08-11+unicode-17.0.0",
  "id": "quick-start/two-lines",
  "request": {
    "source": "",
    "size": {"inline": 1000, "block": 1000},
    "frame": "full-em",
    "clusters": [],
    "line_extent": 4000
  }
}
```

The response repeats `protocol`, `spec`, and `id`, replacing `request` with `response`.
Mismatch is an input/protocol error, never a conformance difference. Messages are one JSON
object per UTF-8 line; blank lines are ignored.

The committed [`protocol.schema.json`](../../crates/kumihan-conformance/protocol.schema.json)
is the portable format contract. The sample suite and engine are in the same directory.
The CLI validates request and response bodies, closed enum vocabularies, ranges, integer
bounds, UTF-8 cluster coverage, and unknown fields in addition to the envelope version.

## Commands

```text
kumihan-conformance list [SUITE.ndjson]
kumihan-conformance validate [SUITE.ndjson|-]
kumihan-conformance run ENGINE [SUITE.ndjson]
```

With no suite path, `list` and `run` use the built-in suite. `validate` reads stdin unless a
path is provided. `run` starts `ENGINE` once, sends every request to its stdin, closes
stdin, then reads one response per request from stdout.

Exit codes are fixed:

| Code | Meaning |
| --- | --- |
| 0 | every case matched, or validation succeeded |
| 1 | one or more observable results differed |
| 2 | invalid JSON/input, protocol mismatch, process error, or malformed response |

Response IDs must occur in request order and match exactly. A missing, extra, or reordered
response is a protocol error. A valid response whose result differs from `expected` is a
conformance difference.

## Request model

Requests contain original, pre-normalization data:

- `source`, default `size`, explicit default `frame`, and shaped `clusters`;
- each cluster's UTF-8 byte `range`, integer `advance`, and optional size/frame/role;
- `line_extent`;
- ordinary, mandatory, or discretionary `breaks`;
- any of the nine public `constructs`, including shaped ruby/mark/script annotations;
- `tab_stops`, `first_line_indent`, `alignment`, widow minimum, and `writing_mode`; and
- either a dated Style profile or any of the 22 stable dotted settings.

The format never supplies normalized classes, rule IDs, lowering contributions,
feasibility, adjustment stages, or algorithm parameters. An implementation is free to use
a different internal model.

## Response model

A response contains only observable output:

- lines with original source ranges and logical inline/block geometry;
- cluster placements with original attribution, size/frame, local writing mode, and
  transform;
- construct attachments with shaped range or repeated symbol; and
- diagnostics with stable code, severity, input range, and JLReq reference.

All numbers are integers in the caller's units. Array order is significant and stable.

## Included sample engine

`kumihan-sample-engine` implements protocol v1 on top of the Rust API. It is intentionally a
separate executable rather than an in-process adapter, so the repository tests the same
pipe behavior a non-Rust implementation uses.

```powershell
cargo build -p kumihan-conformance --bins
$engine = (Resolve-Path target/debug/kumihan-sample-engine.exe).Path
cargo run -p kumihan-conformance -- run $engine
```

The package has no library target. JSON parsing dependencies remain confined to this
product and never become dependencies or features of `kumihan`.

Suite envelopes carry a non-empty `rules` array beside `request` and `expected`. This is
runner-only provenance used by the coverage gate; `run` strips it before sending the request
to an engine, and protocol requests or responses without `expected` reject the field.

## Coverage and release status

The legacy case corpus and generated inventories remain differential assets while cases are
translated to protocol v1. They currently inventory 106 rules, cover 77, classify two as
editorial and three as non-observable, and record 24 deferrals. A green legacy coverage
subtraction means every rule is covered, explicitly classified with evidence, or deferred;
it does not mean 1.0 is complete.

Before release, every mechanically observable deferral must have a non-empty black-box
case. Editorial guidance and statements no layout result can observe move to explicit
`editorial` or `non-observable` classifications with evidence; empty cases never count as
coverage. An external process must then be able to run the complete suite using only this
protocol.
