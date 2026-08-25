# Security Policy

## Scope

jlreq performs pure computation over in-memory text. It does not read files, open
network connections, execute code, or link native libraries, and the core is `no_std`
([ADR 0001](docs/adr/0001-no-std-no-io-no-font-in-core.md)). The realistic threat is
untrusted text reaching the layout engine: a panic, an unbounded allocation, or
non-termination on adversarial input is a security issue here, because callers embed this
in servers and document pipelines.

Specifically in scope:

- Panics on any `&str` input, including malformed sequences, degenerate advances, and
  pathological ruby or nesting depth
- Unbounded memory growth or non-termination driven by input size or content
- Integer overflow producing incorrect placement rather than a defined error

Composition has deterministic caller-configurable bounds for clusters, break candidates,
constructs, tab stops, and charged search transitions. The protocol runner separately
bounds message and suite bytes, case count, retained stderr, and inactivity time; a refusal
through one of these controls is expected behavior, not a partial result.

Out of scope: a layout result you disagree with. Where JLReq permits alternatives, use an
issue or a conformance case.

## Reporting

Report privately through GitHub's ["Report a vulnerability"][advisories] flow rather than a
public issue. Include the input, the advances supplied, and the observed behavior.

Expect an acknowledgement within seven days.

## Supported versions

The prepared 0.1.x line receives security fixes. Before the first publication, reports
against the prepared 0.1.0 tree and `main` follow the same policy.

| Version | Supported |
| --- | --- |
| `0.1.x` | Supported |
| `< 0.1.0` development snapshots | Unsupported |

[advisories]: https://github.com/P4suta/jlreq/security/advisories/new
