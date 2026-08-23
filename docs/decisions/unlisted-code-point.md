# Reading: the class of a code point Appendix A does not list

- Applies to: `jlreq_class::resolve`
- Standing: `Unstated`
- JLReq: §3.9.2, §3.2.4, §3.2.6

## The silence

§3.9.2 says, of the class lists it and JIS X 4051 publish:

> JIS X 4051 also provides similar character classes but that are slightly different from
> this document. Furthermore JIS X 4051 states that it is implementation-defined how to
> handle characters that are not explicitly mentioned, e.g. whether they should belong to
> either class or not.

JLReq records that and does not replace it. Appendix A enumerates 1133 keys and the
`Unified_Ideograph` property covers 101 996 code points that §A.19 deliberately leaves to
the character database, so most of Unicode reaches this case: Devanagari, Hangul, emoji,
mathematical alphanumerics, and every script JLReq never considered.

## The reading

`resolve` answers **cl-27 (Western characters) when the occurrence declares
`Frame::Proportional`, and cl-19 (ideographic characters) otherwise**.

`classify` still answers `Classified::Unlisted`, because that is the fact. The reading
applies in `resolve` and only there, and every answer it produces carries
`Standing::Unstated`.

## Why

The reading extends the one distinction JLReq itself draws over text its tables do not
enumerate. §3.2.4 sets full-width and fixed-width Western characters and European numerals
"as quasi Japanese characters", spaced against hiragana, katakana and ideographs exactly as
an ideograph is. §3.2.6 gives Western text set with a proportional font the composition
rules of cl-27. Neither section works from a list; both work from how the character was
set, which is precisely the fact an unlisted occurrence still carries.

So the answer is not invented: it is the frame axis, which the specification already makes
decisive over unenumerated text, applied where the table stops. An unlisted occurrence on
the ideographic frame behaves as an ideograph, which is what a Japanese compositor does with
an unfamiliar full-width glyph; an unlisted occurrence on a proportional frame is Western
text by §3.2.6's own criterion.

The alternatives are worse for stated reasons. Refusing to answer leaves `resolve` with no
answer, which is what it exists to have. Answering cl-19 unconditionally puts a proportional
Latin glyph the shaper produced into the class §3.2.6 excludes it from. Deriving the class
from a Unicode property other than `Unified_Ideograph` — from `Script`, or from
`East_Asian_Width` — would make jlreq's answer depend on a property JLReq never cites and
would change with every Unicode revision, which is the coupling
[ADR 0008](../adr/0008-classification-is-a-function-of-an-occurrence.md) keeps to the two
places the specification asks for it.

## What would change it

A revision of §3.9.2 that decides the case rather than recording that JIS leaves it open. A
`Question::UNLISTED_CODE_POINT` in the generated policy space: this reading is what applies
when the caller answers nothing.

An adopter who needs a different answer already has one without a policy question, because
`classify` reports `Classified::Unlisted` and the caller may decide for themselves. That is
the point of `classify` and `resolve` being two functions.
