## What this changes

<!-- One or two sentences. What behavior is different after this lands? -->

## Specification basis

<!--
For anything that changes layout behavior, cite what makes it correct: the JLReq section,
the JIS X 4051 clause, or the ADR. "It looks better" is not a basis; if the specification
permits both, this should be an option rather than a new default (see ARCHITECTURE.md).
Delete this section for changes that do not touch layout.
-->

## Conformance

- [ ] Every observable rule this touches has a protocol-v1 case naming its JLReq section
- [ ] Where JLReq permits alternatives, each permitted outcome is recorded
- [ ] A disagreement with LaTeX's `jlreq` class or a browser is documented with reasoning

## Checks

- [ ] `just ci` passes locally
- [ ] The layout core gained no `std`, I/O, font, or floating-point dependency
      (`just purity`, `just no-std`, `just wasm`)
- [ ] No `allow` or `ignore` was added to make a gate pass

<!--
If a gate was changed rather than satisfied, say why here. That is sometimes right, and it
always deserves a sentence.
-->
