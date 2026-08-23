# Language-independent conformance protocol

`jlreq-conformance` is a binary-only product. It runs a suite against an external engine
process through NDJSON; an engine in Rust, C++, JavaScript, Python, or any other language
sees the same black-box contract.

## Versioning

Every request and response envelope must contain exactly the protocol and specification
line it implements:

```json
{
  "protocol": "jlreq.conformance/1",
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

The committed [`protocol.schema.json`](../../crates/jlreq-conformance/protocol.schema.json)
is the portable format contract. The sample suite and engine are in the same directory.
The CLI validates request and response bodies, closed enum vocabularies, ranges, integer
bounds, UTF-8 cluster coverage, and unknown fields in addition to the envelope version.

## Commands

```text
jlreq-conformance list [SUITE.ndjson]
jlreq-conformance validate [SUITE.ndjson|-]
jlreq-conformance run ENGINE [SUITE.ndjson]
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

`jlreq-sample-engine` implements protocol v1 on top of the Rust API. It is intentionally a
separate executable rather than an in-process adapter, so the repository tests the same
pipe behavior a non-Rust implementation uses.

```powershell
cargo build -p jlreq-conformance --bins
$engine = (Resolve-Path target/debug/jlreq-sample-engine.exe).Path
cargo run -p jlreq-conformance -- run $engine
```

The package has no library target. JSON parsing dependencies remain confined to this
product and never become dependencies or features of `jlreq`.

Suite envelopes carry a non-empty `rules` array beside `request` and `expected`. This is
runner-only provenance used by the coverage gate; `run` strips it before sending the request
to an engine, and protocol requests or responses without `expected` reject the field.

## Independent reference engines

`jlreq-conformance` is a protocol, not a Rust harness: it exchanges NDJSON with whatever
process is named on the command line, so the claim that it is language independent is
falsifiable. [`engines/`](../../engines/) holds independent implementations of that protocol
that make the claim testable. They are not products: each is outside the Cargo workspace,
[ADR 0022](../adr/0022-unified-public-crate-and-process-conformance.md)'s public-surface
gates (`purity`, `api`, `direction`, `derive`, `generate`) never see them, and none is a
`jlreq` dependency in either direction. The first is [`engines/ocaml/`](../../engines/ocaml/README.md);
`engines/racket/` follows the same shape.

They exist for two reasons:

- **the protocol claims to be language independent**, and a second implementation in a
  language with a different runtime, a different native integer type, and a different
  standard library is the only way to find out;
- **N-version cross-checking**: three implementations independently answering all
  eighty-nine built-in cases the same way is evidence the answer is right, not evidence that
  one implementation's mistake was copied into another.

### The runner contract

The suite runner starts an engine exactly once, with no arguments, from a working directory
it never discloses: `Command::new(engine)`. An engine therefore cannot resolve a data file at
run time — every reference table it needs must be embedded in the executable at build time —
and it communicates only through stdin/stdout NDJSON, exactly as the sample engine does. A
single compiled, argument-free executable is the target shape for every language a reference
engine is written in, Windows included.

### The integer contract

Every reference engine computes in exact integers and follows the same two-layer contract the
Rust implementation does ([ARCHITECTURE.md](../../ARCHITECTURE.md#input-invariants)):

- every scalar that crosses the protocol boundary is a bounded `i32` in the caller's unit,
  matching the schema's `i32` range (`[-2147483648, 2147483647]`); a floating-point value
  anywhere in a response is a guaranteed conformance failure, because comparison is
  structural JSON equality and `1` is never `1.0`;
- intermediate sums, products, and the line-breaking cost function stay in a wider integer —
  Rust's `i64`, or the closest equivalent the host language offers with the same width and
  the same wraparound point — combined with `checked` or `saturating` arithmetic, never with
  a silently wrapping operator;
- `/` and `%` truncate toward zero (Rust's operators; `-7 / 2` is `-3`, `-7 % 2` is `-1`), so
  an engine only has to reach for a truncating primitive, not build one — most host languages'
  native division already truncates;
- a `usize`-shaped subtraction (a byte offset minus a byte offset, a remaining-width
  computation) saturates at zero rather than going negative, mirroring `usize::saturating_sub`;
  this is the likeliest place to get the port wrong, because in most host languages the plain
  subtraction operator does not stop there and has to be routed through an explicit helper
  everywhere the source subtracts one offset from another.

This section is the single source for that contract. Every reference engine states, in its own
arithmetic module, that this is the section it is implementing rather than restating the
reasoning locally, so the contract is amended in exactly one place.

### The independence rule

**May be read and relied on** — the public contract:

- [`protocol.schema.json`](../../crates/jlreq-conformance/protocol.schema.json), the format
  contract;
- this document;
- the field names and value vocabularies the sample engine puts on the wire;
- everything under `spec/` — the W3C snapshot, the derived tables, and the captured
  matrices — which is the *specification*, not an implementation of it;
- the published legend-token grammar for Tables 1 through 6.

**May not be read into a reference engine** — the Rust implementation:

- the layout logic under `crates/jlreq/src/` (`pipeline.rs` above all);
- `crates/jlreq/src/generated/`, and the xtask code generators that write it.

A reference engine builds its tables from `spec/derived/*.tsv` and `spec/captured/*.tsv`
directly; it does not read the Rust-generated modules those files also produce. Where two
engines transcribe the same hand-keyed matrix, they read it from opposite locales — the Rust
sample engine reads `table*.en.tsv`, an OCaml or Racket engine reads `table*.ja.tsv` — so
agreement is evidence about the transcription and not an artifact of copying the same
keystrokes twice ([ADR 0009](../adr/0009-generated-data-and-attested-transcription.md)).

Where two engines disagree, the disagreement is settled by returning to JLReq and to `spec/`,
and the resolution is recorded in `docs/decisions/`. It is never settled by reading the other
engine's source and copying its answer, and never by majority vote among the engines running
at the time: a rule that is observable in the Rust engine but written down nowhere else is
exactly the finding a second implementation exists to surface.

### Milestone gating

A reference engine reaches the full eighty-nine-case suite through an ordered sequence of
disjoint milestones rather than in one step; each engine's `milestones/` directory holds the
partition and a `CURRENT` file naming the milestone the tree currently claims. The
corresponding CI job runs the cumulative suite through `CURRENT`, so `CURRENT` is what a merge
is actually held to, and advancing it is part of the pull request that makes the next
milestone pass. See `engines/ocaml/README.md` for the concrete commands and the current
milestone table.

## Coverage and release status

The generated inventory contains 106 rules: the built-in protocol-v1 suite has 89 non-empty
black-box cases that directly name all 100 mechanically observable rules, three rules are
classified editorial, three are classified non-observable, and none is deferred.

The coverage gate requires every mechanically observable rule to have a non-empty case.
Editorial guidance and statements no layout result can observe carry explicit
`editorial` or `non-observable` classifications with evidence; empty cases never count as
coverage. The bundled sample engine runs the complete protocol-v1 suite as an external process
using only this contract.
