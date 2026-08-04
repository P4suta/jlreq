# ADR-0006: the conformance suite is a deliverable, not an internal test

- Status: accepted
- Date: 2026-08-05

## Context

The reason nobody has written this library is not that the rules are hard. It is that the
rules exist as prose in JLReq and JIS X 4051, and as behavior inside TeX, browsers, and
InDesign — and there is no executable statement of what correct means. Anyone starting
work has to re-derive the rules from prose and then has no way to check the result against
anything.

That missing artifact is worth more than the implementation. An implementation helps
people who adopt this crate. An executable specification helps everyone, including the
browser engineers and the Typst maintainers who will never depend on us.

If the suite is written as internal unit tests it will be shaped by our internal
structure — organized around our modules, asserting our types — and it will be useless to
anyone else.

## Decision

`jlreq-conform` is a published crate, not a `tests/` directory.

Cases are addressed to the specification, not to our code: each carries the JLReq section
it exercises, and the suite is organized by specification structure so a reader can find
"what does this project believe section 3.1.3 requires" without reading any Rust.

A case is data — text, advances, expected placement — evaluated against a trait. This
project supplies one implementation of that trait. Another implementation can supply
another and run the same suite.

Where JLReq permits alternatives, the suite records every permitted outcome rather than
picking one, and the corresponding knob is a caller-visible option
([ARCHITECTURE.md](../../ARCHITECTURE.md)). Where our reading of the specification differs
from LaTeX's `jlreq` class or from a browser, the case records the disagreement and the
reasoning instead of quietly matching whichever we tested against last.

## Consequences

Publishing our disagreements is uncomfortable and correct. A case that says "we read 3.1.3
this way, Chrome does otherwise, here is why" is more useful to the ecosystem than a green
test suite that hides the question.

Every rule needs a case before it is considered implemented
([CONTRIBUTING.md](../../CONTRIBUTING.md)). This slows the first milestone and pays back
from the second onward, because the suite becomes the specification the implementation is
written against rather than a description of what the implementation happens to do.

The suite is verifiable from the start: [ADR 0002](0002-caller-supplied-metrics.md) makes
a case pure data, and [ADR 0005](0005-integer-layout-units.md) makes the expected value an
exact integer. Neither a font nor a tolerance is involved, so a disagreement between two
implementations is always resolvable by reading the specification.
