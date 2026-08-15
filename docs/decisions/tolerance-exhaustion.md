# Reading: what `compose` does when `Search::Optimal`'s own `tolerance` admits no complete arrangement

- Applies to: `jlreq_line::compose` (`Search::Optimal`)
- Standing: `Unstated`
- JLReq: §3.8 (silence), ADR-0010

## The silence

`docs/design/api-spine.md` states `Search::Optimal`'s own contract in one sentence:
"Minimize total demerits over the paragraph, discarding any line worse than `tolerance`."
That sentence answers what a feasible arrangement is scored by and which lines a caller's
own `tolerance` excludes from consideration. It does not answer what happens when every
complete arrangement — every sequence of breaks reaching from the paragraph's own start to
its own end — needs at least one line `tolerance` excludes. JLReq itself states no
paragraph-level objective at all (`docs/decisions/adjustment-preference.md`'s own silence),
so it has nothing to say about a caller-supplied numeric knob the specification does not
name either.

ADR-0010 states two things this reading must hold simultaneously and that pull against each
other on their face: "the classic infinite-penalty failure cannot occur" — no forbidden
break is ever taken to avoid a worse outcome — and "composition never refuses to produce
lines... because every real adopter must render something." A caller's own `tolerance` can
make the first of those into an apparent dead end: if satisfying it requires taking a break
kinsoku itself forbids, ADR-0010 already refuses that path, and if nothing else is left,
the second half of ADR-0010 still requires an answer.

## The reading

`compose_optimal` runs the dynamic program (`crate::compose::run_dp`) twice at most, never
more:

1. First, restricted to edges whose own `Badness` does not exceed the caller's own
   `tolerance` — literally "discarding any line worse than tolerance," and the only pass
   that ever runs when it succeeds.
2. Only if that leaves the paragraph's own end unreachable — every complete arrangement
   needs at least one line no admitted edge reaches it through — `compose_optimal`
   re-minimizes once more over the full, un-pruned edge set, `tolerance` set aside for this
   one pass. The candidates it chooses among are unchanged either way: exactly the breaks
   `Feasible::compute` already permits, so nothing forbidden is ever taken to satisfy a
   tolerance a caller happened to set too strictly. `Composition::violations` still reports
   every line the fully-drained ladder could not fit, in either pass, so a caller reading
   only the violations list cannot tell which pass produced them — the two differ only in
   what the *search* was willing to consider, never in what a caller is told about the
   result.

The second pass is never merely likely to succeed; it always does, whenever the paragraph
has at least one item. `compose_first_fit`'s own chosen sequence of breaks is always a
complete, valid path through the same un-pruned edge set `run_dp`'s own second pass
searches (`run_dp`'s own doc states the scanning-window argument this depends on), so the
second pass's own minimum is never worse than `FirstFit`'s own total under the same
`Preference`. The fallback is consequently a strict minimization over a graph already proven
non-empty, not a second, weaker algorithm invoked only when the first gives up — a caller
who reads `Composition::demerits()` after tolerance exhaustion is reading a genuinely
searched answer, at least as good as the one `Search::FirstFit` would have produced for the
identical paragraph.

## Why

Three shapes were open, named in the round's own brief: a `FirstFit` fallback, a
progressive relaxation of `tolerance`, and a least-bad path that ignores `tolerance`. This
reading takes the third for two reasons neither of the other two share.

A `FirstFit` fallback would silently answer `Search::Optimal` with the greedy algorithm it
does not name — precisely the stub this project's own discipline forbids of the variant
itself (`crate::compose::Search`'s own doc), and precisely as wrong here: a caller who
explicitly asked for the paragraph-level minimum would receive the single-candidate-per-line
answer instead, with no signal in the return type that anything less than the requested
search ran.

A progressive relaxation (raise `tolerance` by some step and retry, repeating until a path
exists) trades one open question for two: the step size is a magic constant this round's own
C5 already argues against introducing elsewhere, and the number of retries is a second one.
Re-minimizing once at full strength answers both without inventing either — there is no
schedule to justify because there is no schedule.

The least-bad-path reading also composes cleanly with the reachability argument
`crate::compose::compose_optimal`'s own doc already needs for a different reason (the
FirstFit-versus-Optimal experiment this round runs): once it is established that the
un-pruned graph is always searchable and always at least as good as `FirstFit`, running that
exact search a second time, only on tolerance exhaustion, costs nothing conceptually new —
it is the same function, called once more with `tolerance: None`.

## What would change it

A revision of JLReq that states a paragraph-level objective, or a published errata that
states what an implementation should do when a caller's own quality floor cannot be met by
any legal arrangement, would settle this outright — nothing here would survive a specific
statement of intent from the specification, because there is none to conflict with today.

Short of that: evidence that real adopters need the specific number of tolerance-violating
lines minimized, rather than only the total demerits, would argue for a reading that
threads a partial tolerance through the fallback pass instead of discarding it outright —
untried here because no such requirement has surfaced.

The conformance suite carries no case exercising this reading yet
(`docs/decisions/README.md`'s own rule: every reading here belongs in the suite with all of
its alternatives), because ADR-0006 makes conformance cases a separately authored phase from
this round's own implementation — stated here rather than left to look met.
