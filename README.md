# kumihan

Japanese line composition (組版) as a library, following the W3C
[Requirements for Japanese Text Layout (JLReq)][jlreq] and JIS X 4051.

> **Status: foundation.** The design is frozen — twenty decision records and a published
> API spine ([docs/design/api-spine.md](docs/design/api-spine.md)) — and the two crates
> the rest of the workspace speaks through, `jlreq-unit` and `jlreq-spec`, are
> implemented, alongside the quality gates that keep the design mechanical rather than
> aspirational. No layout logic is implemented yet, and the generated specification tables
> are not emitted yet. See [ROADMAP.md](ROADMAP.md).

## Why this exists

Correct Japanese text layout is not a library anywhere. It exists as the `jlreq` LaTeX
class, inside browser engines, and inside InDesign — three implementations, none of which
you can call. Rust has crates for furigana dictionary lookup and kana conversion, and
nothing for line composition.

The visible consequence: text laid out by a Rust application breaks lines wherever the
Unicode line breaking algorithm (UAX #14) permits, which puts `、` and `。` at the start
of a line. That is wrong in a way any Japanese reader notices immediately, and there is
currently no crate to fix it.

## What it does

Given text, a per-character advance, and the character frame (字幅) that advance covers,
produce placement instructions:

- **Character classes** — the 30 JLReq classes (cl-01 … cl-30) that drive every other
  rule, determined for an occurrence rather than for a code point
- **Kinsoku (禁則)** — characters prohibited from starting or ending a line
- **Mojikumi (文字組み)** — spacing between punctuation, brackets, and ideographs
  (equivalent to the CSS `text-spacing-trim` property, which JLReq specifies)
- **Line adjustment** — oikomi (追い込み) and oidashi (追い出し), with hanging punctuation
  (ぶら下げ) as a stage of the same ladder
- **Inline constructs** — ruby, tate-chu-yoko (縦中横), emphasis dots, warichu (割注), and
  five more that lower the same way
- **Vertical writing** — as a writing direction, not a separate code path

## What it deliberately does not do

It does not load fonts, shape glyphs, rasterize, or touch the filesystem. It sits between
the text pipeline you already have and your renderer:

```text
   your application / Typst / Parley / PDF writer / game engine
                            ▲
                        kumihan          ← line composition only
                            ▲
     ICU4X (UAX #14 break opportunities) / HarfRust (shaping)
```

The caller supplies each character's advance; the library returns positions and spacing.
That boundary is what makes the core `no_std`, free of floating point, and testable
entirely in CI without a single font file. See [ARCHITECTURE.md](ARCHITECTURE.md) and the
[decision records](docs/adr/).

## Crates

| Crate | Responsibility |
| --- | --- |
| `jlreq-unit` | Quantities, axes, and the item vocabulary |
| `jlreq-spec` | Specification addresses, provenance, and the policy space |
| `jlreq-class` | JLReq character class of an occurrence (cl-01 … cl-30) |
| `jlreq-spacing` | Inter-class spacing amounts (mojikumi) |
| `jlreq-line` | Line composition: kinsoku, the adjustment ladder, hanging punctuation |
| `jlreq-inline` | Ruby, tate-chu-yoko, emphasis dots, warichu, and the other constructs |
| `jlreq` | Facade over the above |
| `jlreq-conform` | Conformance suite, mapped one-to-one onto JLReq sections |

## Development

```sh
mise install        # toolchain and gate tooling
mise run hooks      # install the git hooks
just                # list the available commands
just check          # fast inner-loop gates
just ci             # every gate CI runs
```

## License

Dual-licensed under [MIT](LICENSES/MIT.txt) or [Apache-2.0](LICENSES/Apache-2.0.txt), at
your option. The repository is [REUSE][reuse]-compliant.

[jlreq]: https://www.w3.org/TR/jlreq/
[reuse]: https://reuse.software/
