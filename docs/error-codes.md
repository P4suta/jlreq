<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Stable error and diagnostic codes

The code is the compatibility key; prose messages may become clearer in patch releases.
`LayoutError` and facade diagnostics cover automatic text/font processing.
`jlreq_core::InputError`, `StyleError`, `ComposeError`, and core `Diagnostic` cover
pre-shaped composition. `just repository` compares this table with every literal in both
handwritten public-library sources in both directions.

## High-level font and option errors

| Code | Meaning |
| --- | --- |
| `font.invalid` | Font bytes or the selected TTC face index are invalid. |
| `font.none-registered` | Layout was requested without a registered face. |
| `font.unknown-id` | Primary or fallback configuration names an unregistered font ID. |
| `font.too-many-ids` | Registered faces cannot be represented by a public font ID. |
| `font.system-family-not-found` | Opt-in system discovery found no face for the requested family. |
| `layout.invalid-option` | A numeric, tag, language, tab, feature, or variation option violates its invariant. |

OpenType tags use this single option code when they are not exactly four bytes in
`0x20..=0x7e`, contain a leading/interior space, or use non-space data after trailing
padding. Invalid hit-test coordinates are also option failures; an invalid UTF-8 caret or
selection boundary instead returns `None` or an empty rectangle list because result
queries do not mutate layout state.

## High-level document errors

| Code | Meaning |
| --- | --- |
| `document.invalid-span-range` | A span range is empty, out of bounds, or not on UTF-8 boundaries. |
| `document.overlapping-spans` | Two span styles overlap. |
| `document.invalid-paragraph-style-range` | A paragraph-style range is empty, out of bounds, or not on UTF-8 boundaries. |
| `document.overlapping-paragraph-styles` | Two paragraph styles overlap. |
| `document.paragraph-style-splits-paragraph` | A paragraph-style range cuts a paragraph instead of containing it. |
| `document.invalid-break` | An authored break offset is out of bounds or splits UTF-8. |
| `document.conflicting-break` | The same offset is both mandatory and prohibited. |
| `document.invalid-construct-range` | A typed structure has an invalid source range. |
| `document.empty-ruby-annotation` | Ruby annotation text is empty. |
| `document.invalid-ruby-run` | An explicit ruby association has an invalid range. |
| `document.group-ruby-run-count` | Explicit group ruby does not contain exactly one association. |
| `document.incomplete-ruby-runs` | Explicit ruby associations do not cover both base and annotation. |
| `document.empty-reference-mark` | A reference-mark annotation is empty. |
| `document.empty-script-annotation` | A subscript or superscript annotation is empty. |
| `document.invalid-furawake-columns` | Furawake requests fewer than two columns. |
| `document.invalid-jidori-cells` | Jidori requests zero cells. |
| `document.construct-crosses-paragraph` | One structure crosses a mandatory paragraph boundary. |
| `document.span-splits-grapheme` | A styled span endpoint splits an extended grapheme. |
| `document.mono-ruby-cluster-count` | Automatic mono ruby cannot map its base and annotation clusters one-to-one. |

## High-level resource errors

| Code | Meaning |
| --- | --- |
| `limit.input-bytes` | UTF-8 input exceeds the configured byte maximum. |
| `limit.fonts` | Registered faces exceed the configured count. |
| `limit.font-bytes` | Total registered font data exceeds the configured byte maximum. |
| `limit.paragraphs` | The input contains too many paragraphs. |
| `limit.runs` | Itemization would create too many shaping runs. |
| `limit.glyphs` | Shaping would produce too many glyphs. |
| `limit.constructs` | The typed document has too many inline structures. |
| `limit.core-operations` | Core composition exceeds the facade's operation budget. |

## Input errors

| Code | Meaning |
| --- | --- |
| `input.invalid-size` | A size is not positive on both axes. |
| `input.cluster-out-of-range` | A cluster is empty or outside its source. |
| `input.invalid-utf8-boundary` | A range endpoint splits a UTF-8 code point. |
| `input.negative-advance` | A shaped cluster advance is negative. |
| `input.overlapping-clusters` | Cluster coverage overlaps. |
| `input.uncovered-text` | Cluster coverage leaves source bytes uncovered. |
| `input.cluster-covers-multiple-keys` | One non-proportional cluster hides multiple JLReq keys. |
| `input.empty-construct` | A ruby or other structure has an empty base. |
| `input.ruby-without-runs` | Ruby has no base-to-annotation run. |
| `input.group-ruby-run-count` | Group ruby does not contain exactly one run. |
| `input.invalid-ruby-base-run` | Ruby base runs do not partition the declared base. |
| `input.invalid-ruby-annotation-run` | Ruby annotation runs do not partition shaped annotation. |
| `input.incomplete-ruby-runs` | Ruby runs do not cover both streams completely. |
| `input.invalid-line-extent` | The line extent is not positive. |
| `input.invalid-indent` | The first-line indent leaves no positive measure. |
| `input.break-splits-cluster` | A declared break is not a shaped-cluster boundary. |
| `input.duplicate-break` | More than one break is declared at an offset. |
| `input.construct-out-of-range` | A structure is empty or outside the source. |
| `input.construct-splits-cluster` | A structure endpoint splits a cluster. |
| `input.crossing-constructs` | Structure ranges cross instead of nesting or remaining disjoint. |
| `input.break-inside-construct` | A break violates the selected structure's break model. |
| `input.mono-ruby-run-shape` | Mono ruby does not map one base cluster per run. |
| `input.ruby-run-splits-cluster` | A ruby run endpoint splits a base cluster. |
| `input.invalid-furawake-columns` | Furawake has no columns. |
| `input.invalid-furawake-line-gap` | Furawake has a negative line gap. |
| `input.furawake-split-count` | Furawake break declarations do not match its columns. |
| `input.furawake-empty-subline` | A furawake declaration creates an empty subline. |
| `input.invalid-jidori-cells` | Jidori has no cells. |
| `input.invalid-tab-stop` | A tab stop position is not positive. |
| `input.duplicate-tab-stop` | Tab stop positions are not strictly increasing. |
| `input.tab-stop-outside-line` | A tab stop is beyond the usable line extent. |
| `input.insufficient-tab-stops` | A mandatory-line partition has more line tabs than stops. |

## Style errors

| Code | Meaning |
| --- | --- |
| `style.very-strict-relaxation` | Very-strict kinsoku is combined with relaxation. |
| `style.very-strict-grouped-numeral` | Very-strict kinsoku permits a grouped-numeral break. |

## Composition resource errors

| Code | Meaning |
| --- | --- |
| `compose.cluster-limit` | The paragraph exceeds the configured cluster limit. |
| `compose.break-candidate-limit` | The paragraph exceeds the break-candidate limit. |
| `compose.construct-limit` | The paragraph exceeds the structure limit. |
| `compose.tab-stop-limit` | The paragraph exceeds the tab-stop limit. |
| `compose.transition-limit` | Exact search exceeds the configured transition budget. |

## Layout diagnostics

| Code | Meaning |
| --- | --- |
| `font.missing-glyph` | No registered face covers a complete grapheme; the primary `.notdef` preserves its range. |
| `font.unknown-family` | A span requested a family no registered face declares; the library fallback order was used. |
| `layout.overfull` | A complete line remains wider than its measure after permitted adjustment. |
| `layout.widow` | The complete layout cannot meet the requested final-line cluster minimum. |
