<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Stable error and diagnostic codes

The code is the compatibility key; prose messages may become clearer in patch releases.
`InputError` reports invalid caller data, `StyleError` reports contradictory settings,
`ComposeError` reports an atomic resource refusal, and `Diagnostic` describes a complete
layout that was necessarily degraded. `just repository` compares this table with every
literal in the handwritten product source in both directions.

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
| `layout.overfull` | A complete line remains wider than its measure after permitted adjustment. |
| `layout.widow` | The complete layout cannot meet the requested final-line cluster minimum. |
