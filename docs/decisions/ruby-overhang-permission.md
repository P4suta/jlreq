<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: which space a ruby reading may hang over, where §3.3.8 and Table 1 differ

- Applies to: the ruby overhang round in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Unstated`
- JLReq: §3.3.8, §3.3.3, §B.1, §B.2#7, §A.09, §A.10, §A.15, §A.16
- Observed by: `just census ruby` (37,030 requests), by the second engine for the first two
  readings and by the third engine's convergence on the same census for the three below
  them

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

**The allowances are a list of separate permissions, and three of them name one side of
the object.** §3.3.8 introduces them as "the general rules", and they are not one rule with
exceptions: three name a *character* the reading may go over, one names a *space*, and one
names a sum of the two. Three name a configuration rather than a neighbor:

> The ruby character may overhang the base characters and overhang the half em spacing
> which is added after closing brackets (cl-02), full stops (cl-06) or commas (cl-07), set
> before the target ruby object […] Also, the ruby character may […] hang over the half em
> spacing which is added before opening brackets (cl-01), set after the target ruby object

and, two rules later,

> When the adjacent character is one of the closing brackets (cl-02), full stop (cl-06) or
> comma (cl-07) after the ruby object, or one of the opening brackets (cl-01) before the
> ruby object, the ruby text may overhang the adjacent base character […] Note that the
> overhang must not go beyond the closing brackets (cl-02), the full stop (cl-06), the
> comma (cl-07) or the opening brackets (cl-01) itself.

The first grants the *space* beside a mark standing on one side; the second grants the
*mark* standing on the other. Neither is stated in the mirrored configuration, and the
section does not say whether the unnamed half of each pair is denied or merely unmentioned.
Table 1 states its `hang` cells on both sides of every one of these coordinates, so the
matrix reads as though the permissions were symmetric while the prose is sided.

**"The full-width size of the ruby characters" does not say which ruby characters.** Every
allowance is measured in that quantity, and a ruby construct offers three candidates for
it: the size the caller declared for the construct's annotation, the largest character of
the *run* that is doing the overhanging, and the largest character anywhere in the
compound. §3.3.3 makes the ruby size half the base characters' "in principle", which is a
default rather than a constraint, and the protocol shapes every character with metrics of
its own. The section is written as though there were one number.

**A middle dot carries two statements and no rule for choosing between them.** §3.3.8:

> When the adjacent character is one of the middle dots (cl-05), the ruby text may overhang
> the middle dots, in principle, up to the full-width size of a ruby character. But if
> there is any reduction of spacing before and after the middle dots as a result of the
> line adjustment, the amount of the extension shall be up to the amount of spacing after
> the middle dots plus 1/2 a ruby character size […]

"In principle" and "but if there is any reduction" are two expressions for one allowance,
and the section does not say what the second evaluates to where nothing was reduced, or
which of the two an engine asks first.

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

**An allowance is available in the configuration §3.3.8 states it for and is not
mirrored.** The four cases the two sided rules produce are four different answers:

| The neighbor | Where it stands | What the reading may go over |
| --- | --- | --- |
| cl-01 | before the object | the bracket itself, and not the half em before it |
| cl-01 | after the object | the half em before it, and not the bracket itself |
| cl-02, cl-06, cl-07 | before the object | the half em after it, and not the mark itself |
| cl-02, cl-06, cl-07 | after the object | the mark itself, and not the half em after it |

The two rows that deny the space are the section's own closing note — the overhang "must
not go beyond the […] brackets […] itself" — and the two that grant it are the earlier
rule. Nothing is added in the mirrored direction.

**"The full-width size of the ruby characters" is the largest character of the run that is
doing the overhanging.** It is measured neither at the size the construct declared for its
annotation nor at the largest character somewhere else in the compound. A jukugo compound
whose second run is set larger than its first therefore hangs each run over the neighbor
beside it by that run's own maximum, and the two ends of one compound can carry different
allowances.

**A middle dot's allowance is the sum §3.3.8 states for a reduced dot, at every
coordinate.** The amount is the spacing that actually stands beside the dot — after it
where the dot stands before the object, before it where the dot stands after — plus half a
ruby character. It is asked whether or not anything was reduced, and the "in principle"
sentence is never evaluated separately.

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

**The section says "before or after" wherever it means both sides, and does not say it
here.** Its first two rules are stated of "the adjacent character", and the Japanese
rendering of each opens 前又は後ろにくる — *coming before or after*. The three sided rules
name one configuration in both renderings, and the second of them spends a whole sentence
denying the space on the side it granted the mark. A section that has the vocabulary for
"either side", uses it twice, and then writes out one side four times is drawing the
distinction on purpose; mirroring the allowances would make those four clauses say nothing
that "the adjacent character" would not already have said. The four rows above are also
not arbitrary as a set: on each side, exactly one of the mark and the space beside it is
available, so a reading never crosses a mark *and* the space it stands next to.

**A permission measured in "a ruby character" is measured in the characters that are
crossing the boundary.** What §3.3.8 is bounding is how far the reader's eye follows a
reading past its object, and the reading that does the crossing is one run. The declared
annotation size is a request rather than a measurement — the metrics are the caller's
([ADR 0002](../adr/0002-caller-supplied-metrics.md)), and a shaped character need not have
the size the construct asked for — so reading the allowance off the declaration would bound
a distance by a number no character on the line has. Taking the compound's largest
character is worse still: it lets a run three base characters away, which never approaches
this boundary, decide how far this one may hang.

**The reduced-dot expression is the section's own answer at the section's own ruby size.**
§3.3.3 makes a ruby character half a base character in principle. At that size the quarter
em Table 1 states beside a middle dot *is* half a ruby character, so "the amount of spacing
after the middle dots plus 1/2 a ruby character size" evaluates to exactly one ruby
character — the "in principle" sentence's own answer — wherever nothing was reduced. The
two statements are one expression, and the reduced case is the general form of it. Asking
the "in principle" sentence first and the sum only after a reduction would need a rule for
which sentence governs, which §3.3.8 does not state; taking the sum always needs none, and
reproduces both sentences where both are stated. It also matches what the section does two
rules earlier for the bracket case, where the room "is also compressed to the reduced
spacing" rather than being answered from a second sentence.

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

For the third: a revision of §3.3.8 that states each allowance of "the adjacent character"
in the way its first two rules do, or that says outright that the list is exhaustive as
stated, settles it. Until then the four rows above are four conformance cases waiting to be
written, each carrying the mirrored answer as a `disagreements` entry.

For the fourth: a sentence naming whose characters the full-width size is measured in — the
run's, the construct's declaration, or the compound's — settles it, and the observable
difference needs a compound whose runs are not all one size, which no built-in case has.

For the fifth: a revision that folds §3.3.8's two middle-dot sentences into one, or that
says which is asked first, settles it. Evidence that publishers set the "in principle"
amount at a ruby size other than §3.3.3's half would be recorded as a `disagreements` entry
rather than as a change here, because at §3.3.3's own size the two readings are the same
number and there is nothing yet to disagree about.
