<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: which space a ruby reading may hang over, where §3.3.8 and Table 1 differ

- Applies to: the ruby overhang round in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Unstated`
- JLReq: §3.3.8, §B.1, §B.2#7, §A.09, §A.10, §A.15, §A.16
- Observed by: `just census ruby` (37,030 requests)

## The silence

Ruby is the one subject where the matrices and the prose overlap most. Sixty of Table 1's
cells carry §B.1's `hang` annotation, and every rule §3.3.8 states has a cell. Where the two
say different things about the same coordinate, nothing says which of them an engine reads.

**§3.3.8 rule 2 names its neighbor two ways at once.** The rule permits a reading to hang
over an adjacent

> hiragana (cl-15), katakana (cl-16), prolonged sound mark (cl-10) or small kana (cl-11)

which is two *scripts* and two *classes*, spelled in one list as though they were four items
of one kind. Table 1 carries a `ruby hang` cell for each of the four classes. The rule does
not say whether "hiragana" and "katakana" name Appendix A's classes cl-15 and cl-16 or the
Unicode scripts of the same names, and the two readings are not the same set of code points.

**Table 1's `hang` annotation does not say whose em its amount was measured in.** Ten of the
cl-22 and cl-23 cells are marked `<amount> be|af hang`. The side letters say which side of
the boundary the amount sits on. Nothing in the annotation says whether the em the amount is
a fraction *of* belongs to the neighbor or to the ruby object itself, and §3.3.8's own prose
is stated about the space beside the base characters rather than about the object's own
metrics. Whether a reading may go over a space that the ruby object's own em produced is a
question the annotation is not shaped to answer.

## The reading

**§3.3.8 rule 2's kana neighbor is read by script and not by class.** The engine reads
`spec/derived/scripts.tsv` rather than the four class rows. The two readings part at exactly
the marks where the scripts and the classes disagree:

- **U+30FC**, the prolonged sound mark, is **cl-10** — which the rule names — and
  `Script=Common`. A reading is **not** set over it.
- **U+30FD** and **U+30FE**, the katakana iteration marks, are **cl-09** — which the rule
  does **not** name — and `Script=Katakana`. A reading **is** set over them.

`ruby.overhang_kana`'s `jis` answer takes katakana out of the same test, and the script
reading survives that too: U+309D, a *hiragana* iteration mark, still gets a reading over it
under `jis`. Table 1's four kana `ruby hang` cells select nothing at all.

**A `hang` term measured from the ruby object's own em is not a space a reading may go
over.** Where the amount was taken from the *neighbor's* em — the half em after a closing
bracket before the object, the half em before an opening bracket after it, the quarter em
beside a middle dot — the reading goes over it, which is what §3.3.8 describes. Where it was
taken from the *ruby object's* own em — the quarter em at `(cl-22, cl-24)`, `(cl-22,
cl-25)`, `(cl-22, cl-27)` and their three mirrors — it does not. The annotation reads `hang`
in both.

## Why

**The rule's own list is two scripts and two classes, and only one of those readings has a
name for what it is doing.** "Prolonged sound mark" and "small kana" are Appendix A class
names; "hiragana" and "katakana" are the names of writing systems that Appendix A happens to
have classes for. A reader who takes all four as class names has to explain why the rule
bothered to name cl-10 and cl-11 separately when cl-15 and cl-16 would have been the same
kind of citation; a reader who takes the first two as scripts gets a rule with one subject —
*kana* — and two classes named to extend it past what the scripts cover. The script reading
is the one under which the sentence is about something.

**Nothing before M6 could see this.** cl-09 and cl-10 are the same row and the same column in
all six matrices, at every coordinate except those four ruby cells. An engine could carry
either reading through 111,090 census requests and every one of the eighty-nine conformance
cases without the difference surfacing anywhere but here. That is the reason to publish it
rather than leave it in a comment: it is invisible until it is not, and the next independent
engine has no way to derive it.

**A space measured in the ruby's own em is the ruby's, and a reading cannot hang over
itself.** §3.3.8's permission is a permission to encroach on the *neighbor* — the space that
exists because of what stands beside the object. A quarter em at `(cl-22, cl-24)` that was
computed from the ruby object's em is part of what the object occupies on the line, not part
of what the neighbor gave up. Letting a reading hang over it would let the object recover
space it had itself asked for, which no reading of §3.3.8 produces, and would make the
object's reported advance and the space its own em generated two different numbers. The
`hang` annotation is not wrong here; it is under-specified, because §B.1's legend has one
token for a permission whose meaning depends on a fact the cell does not carry.

## What would change it

For the first reading: a revision of §3.3.8 that names its kana neighbor by class alone —
adding cl-09 if the iteration marks are meant, or dropping the script vocabulary — settles
it. The concrete form the evidence would take is a conformance case at U+30FC or U+30FD
carrying both answers as `disagreements`, which is what the suite is for; both readings are
publishable today and only one is selected.

For the second: an `em` column in `spec/captured/table1.*.tsv`, or a §B.1 legend token that
distinguishes a `hang` measured in the neighbor's em from one measured in the object's own,
would move the reading out of `docs/decisions/` and into the transcription — which is where
it belongs, because it is a fact about the matrix rather than about the prose. That change
would be visible to both engines at once and is the one worth asking W3C for.
