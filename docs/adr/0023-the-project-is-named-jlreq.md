# ADR-0023: the project is named jlreq

- Status: accepted
- Date: 2026-08-23
- Amends the naming in
  [ADR 0022](0022-unified-public-crate-and-process-conformance.md); its crate topology —
  one public library, one binary-only conformance package — is unchanged.

## Context

The workspace was called `kumihan`. That word is 組版: Japanese typesetting in general. It
names the whole craft, of which this implementation covers one document — W3C's
*Requirements for Japanese Text Layout* (JLReq), at the single revision
`spec/PROVENANCE.toml` pins. A reader who meets `kumihan` on crates.io learns the subject
area and not the contract; a reader who meets `jlreq` learns the contract exactly, because
the specification's own short name is what the whole tree is derived from and attested
against.

The moment to change it is now. The workspace is unreleased at `0.0.0`, every package
declares `publish = false`, no compatibility contract has been entered into, and no
protocol version has been published to an implementer. The cost of the rename is a
mechanical one this repository's own gates can verify end to end; after a first release it
would be a compatibility event instead.

Nothing blocks the name. `jlreq` is unused on crates.io.

The name is also a return rather than an invention. The pre-1.0 tree was a graph of
`jlreq-class`, `jlreq-unit`, `jlreq-spec`, `jlreq-spacing`, `jlreq-inline`, `jlreq-line`
and `jlreq-conform` crates; [ADR 0022](0022-unified-public-crate-and-process-conformance.md)
collapsed that graph into a single public crate and named the result `kumihan`. What that
ADR decided was the topology, and the topology is right. Only the name it chose for the
unified crate is being revisited, and it is being revisited back into the namespace the
implementation started in.

## Decision

The project is named `jlreq` throughout:

- **Crates.** `kumihan` → `jlreq`, `kumihan-conformance` → `jlreq-conformance`,
  `kumihan-fuzz` → `jlreq-fuzz`. The directories move with them.
- **Binaries.** `kumihan-conformance` → `jlreq-conformance`, `kumihan-sample-engine` →
  `jlreq-sample-engine`.
- **Protocol identifier.** `kumihan.conformance/1` → `jlreq.conformance/1`. The version
  stays `1`: no message, field, or unit changes, so nothing an implementer has to react to
  changes. Only the identifier's own spelling does, and no implementer holds the old one.
- **Copyright.** `2026 kumihan contributors` → `2026 jlreq contributors` in every SPDX
  header and in `REUSE.toml`.
- **URLs.** `repository`, `homepage`, and `documentation` point at `P4suta/jlreq` and
  `docs.rs/jlreq`.

Two identifiers that contain a version-like string are deliberately **not** touched:

- `SPECIFICATION` remains `jlreq-2020-08-11+unicode-17.0.0`, and `Style::jlreq_2020` and
  the `jlreq-2020` profile keep their names. These are JLReq revision identifiers — they
  name which edition of W3C's document, at which Unicode version, the derived data was read
  from. They were correct before this ADR and are unaffected by it.
- The `spec` field and the `protocol` field of a conformance message are different axes.
  `spec` states which revision of the specification an answer is an answer about; `protocol`
  states which revision of this repository's own wire format carries it. They move for
  different reasons and are never to be conflated because both now begin with `jlreq`.

Three classes of text keep the old name, for the reason
[ADR 0022](0022-unified-public-crate-and-process-conformance.md) already gives about the
crate names it retired:

- `CHANGELOG.md`, whose entries record the reasoning that produced the implementation as it
  was reasoned.
- Historical references to retired code — `crates/jlreq-conform/src/kumihan.rs` and the
  `Kumihan` type — in `docs/conformance-deferrals.toml`, `docs/decisions/`, and
  `xtask/src/conform.rs`. Renaming a file that no longer exists would make the citation
  false.
- `typos.toml`'s dictionary entry `kumihan = "kumihan"`, which allows the Japanese
  typesetting vocabulary word 組版 and never named this project.

`spec/snapshot/` is not edited. It is W3C's document and Unicode's data verbatim, protected
byte for byte by the digests in `spec/PROVENANCE.toml`; the two occurrences of the word
inside it are upstream's own text.

Whether to publish a placeholder crate to hold the `jlreq` name on crates.io is outside
this decision. The workspace stays `publish = false` until a release decision is made
([ADR 0022](0022-unified-public-crate-and-process-conformance.md)).

## Consequences

Every generated artifact changes, by design. `xtask/src/generate.rs` writes an SPDX header
into each generated module, and `data/manifest.toml` records the SHA-256 of every generated
file *and of the xtask sources that generated it*, while each `spec/derived/*.tsv` carries
its reader's digest and each `crates/jlreq/src/generated/*.rs` carries its generator's. One
changed word in one copyright line therefore propagates through the whole digest ledger.
That is the ledger working: a byte that decides an output is not allowed to change
silently. All ten derived files, all ten generated modules, and the manifest were rewritten
by `just derive` and `just generate`, never by hand.

The xtask generation-unit constants become `UNIFIED_*` rather than `JLREQ_*`
(`UNIFIED_TABLE1`, `emit_unified_ranged`, and their siblings). `JLREQ_*` would collide in
meaning with `HISTORICAL_UNITS`, the superseded units that write to `crates/jlreq-spacing/`
and friends: after this rename the prefix `jlreq-` names both the unified crate and the
retired split crates, so it can no longer distinguish them. "Unified" is
[ADR 0022](0022-unified-public-crate-and-process-conformance.md)'s own word for what those
units emit, and it stays discriminating whatever the crate is called.

Because `kumihan` is seven characters and `jlreq` is five, formatting is part of the rename
and not a follow-up: shortened lines let `rustfmt` unwrap wrapped expressions, and the
generation gate hashes the xtask sources, so `just fmt` and `taplo fmt` must run *before*
`just derive` and `just generate` or the manifest records a digest that the next formatting
pass invalidates.

Renaming the GitHub repository from `P4suta/kumihan` to `P4suta/jlreq` is done by hand
outside this tree. GitHub redirects the old URLs, so links in published text and existing
clones keep working; the in-tree URLs are updated here so nothing new depends on the
redirect.
