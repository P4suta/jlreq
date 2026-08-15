# ADR-0013: every rule is addressed by the specification's own address

- Status: accepted
- Date: 2026-08-05

## Context

[ARCHITECTURE.md](../../ARCHITECTURE.md) requires that rules be data plus a small evaluator
"so the conformance suite can address individual rules," and
[CONTRIBUTING.md](../../CONTRIBUTING.md) requires that every rule have a case. Neither is
mechanical today, because nothing names a rule.

Four artifacts need to agree about what a rule is. The generated tables need to say which
sentence produced a cell. Every public item's documentation needs to cite what it
implements. A conformance case needs to say what it exercises. The coverage gate needs to
subtract one set from another. If any of the four uses a different identifier space, the
gate compares different alphabets and the promise is empty.

An internal identifier would be the easy choice and would defeat the point.
[ADR 0006](0006-conformance-suite-as-artifact.md) writes the suite for browser engineers
and Typst maintainers who will never read this code; a failure report saying
`kumihan::MiddleDotSum` requires them to read our source to find out what we meant.

JLReq already has an identifier space, and it is the one those readers hold: section
numbers, appendix note ordinals, table cells. It also has hazards. The published HTML
anchors are slugs carrying no section number, and the appendix legend anchors are off by
one from the table numbers they render — `legend_of_table_2` renders "B.1 Legend of Table
1" — while the PDF filenames are off by one in the same direction.

## Decision

One identifier space, and its addresses are the specification's. A rule address is a
section path, optionally with a note ordinal, or a table cell: `3.1.9`, `B.2#3`,
`C.2#5`, `B.1@cl-05,cl-05`. The grammar is fixed, one canonical rendering is used
byte-identically in the tables, in doc comments, and in the case files, and the section
part is validated against an inventory generated from the document's own rendered section
numbers rather than from its anchor slugs, so a citation to a section that does not exist
fails the build and a renumbered specification fails every stale citation at once.

The `#` separating a note ordinal is ours, because JLReq writes "note 7" in prose and gives
its list items opaque identifiers; that is recorded rather than glossed over. A table cell
is a rule, because most cells implement no note and a coverage gate over sections alone
would be discharged by a single case per appendix.

Every answer the library produces carries the rules that produced it, and every rule
carries its address, the sentence it states, and whether that sentence is normative
specification text, this project's published reading of a silence, or an adjudication
between two things the specification says. The last two exist because the specification
does contradict itself in places and does leave holes — emphasis dots (圏点) have no class
and no row in any table, and §3.8.3 declines to state warichu (割注) adjustment at all —
and a library that quietly filled those would be publishing invention as requirement.

The gate is set subtraction in both directions. Every rule in the inventory has a case, and
every rule an answer is attributed to is in the inventory. Both sides are generated or
data, so it is arithmetic rather than judgment.

## Consequences

A failure report is readable by someone who has never seen this code, which is the whole
point of the suite. So is the crate documentation: it becomes an index of JLReq, because
every public item names the sentence it implements.

The tie can be checked for presence, resolution, and closure. It cannot be checked for
correctness — nothing verifies that an item does what the section it cites requires. That
is what the conformance case is for, which is why closure is the gate that matters: the
unverifiable claim always arrives with an executable one attached.

The address space is a one-way door, because it is baked into every published case file.
