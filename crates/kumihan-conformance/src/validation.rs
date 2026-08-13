// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Structural validation for protocol-v1 request and response bodies.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

const REQUEST_FIELDS: &[&str] = &[
    "source",
    "size",
    "frame",
    "clusters",
    "line_extent",
    "breaks",
    "constructs",
    "tab_stops",
    "first_line_indent",
    "alignment",
    "widow_minimum_clusters",
    "writing_mode",
    "style",
];

const SHAPED_TEXT_FIELDS: &[&str] = &["source", "size", "frame", "clusters"];

const STYLE_CHOICES: &[(&str, &[&str])] = &[
    (
        "kinsoku.level",
        &["very-loose", "loose", "strict", "very-strict"],
    ),
    (
        "adjustment.reduction_table",
        &["table-3", "table-4", "table-5"],
    ),
    ("spacing.line_end_punctuation", &["half-em", "solid"]),
    ("spacing.line_end_full_stop_comma", &["preferred", "jis"]),
    (
        "spacing.line_head_opening_bracket",
        &["pattern-1", "pattern-2", "pattern-3"],
    ),
    ("ruby.overhang_kana", &["kana", "jis", "any", "none"]),
    ("ruby.overhang_indent", &["permitted", "prohibited"]),
    ("ruby.alignment", &["nakatsuki", "katatsuki"]),
    ("ruby.group_distribution", &["jis", "flush"]),
    ("ruby.jukugo_layout", &["group", "phonetic"]),
    (
        "kinsoku.iteration_mark_at_line_head",
        &["prohibited", "permitted", "replaced"],
    ),
    ("adjustment.hanging_punctuation", &["none", "hanging"]),
    (
        "kinsoku.grouped_numeral_before_western",
        &["breakable", "unbreakable"],
    ),
    (
        "spacing.sentence_medial_dividing_mark",
        &["solid", "quarter-em"],
    ),
    (
        "adjustment.japanese_latin_expansion_ceiling",
        &["half-em", "third-em", "rigid"],
    ),
    ("adjustment.expansion_order", &["jis", "implementation"]),
    (
        "adjustment.preference",
        &["least-adjustment", "even-texture"],
    ),
    ("adjustment.remainder", &["leading", "trailing"]),
    (
        "classification.unlisted_code_point",
        &["by-frame", "ideographic"],
    ),
    (
        "classification.ambiguous_context",
        &["lowest-class", "highest-class"],
    ),
    (
        "classification.grouped_numeral_qualification",
        &["by-width", "by-role"],
    ),
    ("kinsoku.relaxation_mechanism", &["reclassify", "matrix"]),
];

const PROFILES: &[&str] = &[
    "jlreq-2020",
    "book-2020",
    "magazine-2020",
    "newspaper-2020",
    "jis-reading-2020",
];

pub(crate) fn validate_request(value: &Value) -> Result<(), String> {
    let request = object(value, "request")?;
    only_fields(request, REQUEST_FIELDS, "request")?;
    validate_shaped_text(request, "request")?;
    positive_i32(request.get("line_extent"), "line_extent")?;

    let source = string(request.get("source"), "source")?;
    if let Some(value) = request.get("breaks") {
        for entry in array(value, "breaks")? {
            let entry = object(entry, "break")?;
            only_fields(entry, &["offset", "kind"], "break")?;
            let offset = offset(entry.get("offset"), "break offset")?;
            if offset == 0 || offset >= source.len() || !source.is_char_boundary(offset) {
                return Err("break offset must be an internal UTF-8 boundary".to_owned());
            }
            enum_string(
                entry.get("kind"),
                "break kind",
                &["allowed", "mandatory", "discretionary"],
            )?;
        }
    }
    if let Some(value) = request.get("constructs") {
        for entry in array(value, "constructs")? {
            validate_construct(entry, source)?;
        }
    }
    if let Some(value) = request.get("tab_stops") {
        for entry in array(value, "tab_stops")? {
            validate_tab_stop(entry)?;
        }
    }
    if let Some(value) = request.get("first_line_indent") {
        i32_value(Some(value), "first_line_indent")?;
    }
    if let Some(value) = request.get("alignment") {
        enum_string(
            Some(value),
            "alignment",
            &["start", "center", "end", "justify"],
        )?;
    }
    if let Some(value) = request.get("widow_minimum_clusters") {
        positive_u16(Some(value), "widow_minimum_clusters")?;
    }
    if let Some(value) = request.get("writing_mode") {
        writing_mode(Some(value))?;
    }
    if let Some(value) = request.get("style") {
        validate_style(value)?;
    }
    Ok(())
}

pub(crate) fn validate_response(value: &Value) -> Result<(), String> {
    let response = object(value, "response")?;
    only_fields(response, &["lines", "diagnostics"], "response")?;
    for line in array(required(response, "lines")?, "lines")? {
        validate_line(line)?;
    }
    for diagnostic in array(required(response, "diagnostics")?, "diagnostics")? {
        validate_diagnostic(diagnostic)?;
    }
    Ok(())
}

fn validate_shaped_text(shaped: &Map<String, Value>, name: &str) -> Result<(), String> {
    let source = string(shaped.get("source"), &format!("{name} source"))?;
    validate_size(required(shaped, "size")?)?;
    frame(shaped.get("frame"))?;
    let clusters = array(required(shaped, "clusters")?, "clusters")?;
    let mut expected_start = 0_usize;
    for value in clusters {
        let cluster = object(value, "cluster")?;
        only_fields(
            cluster,
            &["range", "advance", "size", "frame", "role"],
            "cluster",
        )?;
        let (start, end) = range(required(cluster, "range")?, "cluster range")?;
        if start != expected_start || end > source.len() {
            return Err("cluster ranges must cover the source exactly in order".to_owned());
        }
        if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
            return Err("cluster range must use UTF-8 boundaries".to_owned());
        }
        non_negative_i32(cluster.get("advance"), "cluster advance")?;
        if let Some(value) = cluster.get("size") {
            validate_size(value)?;
        }
        if let Some(value) = cluster.get("frame") {
            frame(Some(value))?;
        }
        if let Some(value) = cluster.get("role") {
            enum_string(
                Some(value),
                "cluster role",
                &[
                    "text",
                    "decimal-point",
                    "digit-group-separator",
                    "sentence-medial",
                    "sentence-terminator",
                    "grouped-numeral",
                    "unit-symbol",
                    "quantity-symbol",
                    "formula",
                    "warichu-bracket",
                ],
            )?;
        }
        expected_start = end;
    }
    if expected_start != source.len() {
        return Err("cluster ranges must cover the source exactly in order".to_owned());
    }
    Ok(())
}

fn validate_annotation(value: &Value, name: &str) -> Result<(), String> {
    let annotation = object(value, name)?;
    only_fields(annotation, SHAPED_TEXT_FIELDS, name)?;
    validate_shaped_text(annotation, name)
}

fn validate_construct(value: &Value, source: &str) -> Result<(), String> {
    let construct = object(value, "construct")?;
    let kind = string(construct.get("kind"), "construct kind")?;
    let base_fields = &["kind", "range"];
    match kind {
        "ruby" => {
            only_fields(
                construct,
                &["kind", "range", "ruby_kind", "annotation", "runs"],
                "ruby construct",
            )?;
            validate_source_range(construct, source, "construct range")?;
            enum_string(
                construct.get("ruby_kind"),
                "ruby kind",
                &["mono", "group", "jukugo"],
            )?;
            validate_annotation(required(construct, "annotation")?, "ruby annotation")?;
            for run in array(required(construct, "runs")?, "ruby runs")? {
                let run = object(run, "ruby run")?;
                only_fields(run, &["base", "annotation"], "ruby run")?;
                range(required(run, "base")?, "ruby base range")?;
                range(required(run, "annotation")?, "ruby annotation range")?;
            }
        },
        "emphasis-dots" => {
            only_fields(construct, &["kind", "range", "mark"], "emphasis construct")?;
            validate_source_range(construct, source, "construct range")?;
            one_char(construct.get("mark"), "emphasis mark")?;
        },
        "furawake" => {
            only_fields(
                construct,
                &["kind", "range", "columns", "line_gap"],
                "furawake construct",
            )?;
            validate_source_range(construct, source, "construct range")?;
            positive_u16(construct.get("columns"), "furawake columns")?;
            non_negative_i32(construct.get("line_gap"), "furawake line gap")?;
        },
        "jidori" => {
            only_fields(construct, &["kind", "range", "cells"], "jidori construct")?;
            validate_source_range(construct, source, "construct range")?;
            positive_u16(construct.get("cells"), "jidori cells")?;
        },
        "reference-mark" | "script" => {
            only_fields(
                construct,
                &["kind", "range", "annotation"],
                "annotated construct",
            )?;
            validate_source_range(construct, source, "construct range")?;
            validate_annotation(required(construct, "annotation")?, "construct annotation")?;
        },
        "tate-chu-yoko" | "warichu" | "formula" => {
            only_fields(construct, base_fields, "construct")?;
            validate_source_range(construct, source, "construct range")?;
        },
        _ => return Err(format!("unknown construct kind {kind:?}")),
    }
    Ok(())
}

fn validate_source_range(
    object: &Map<String, Value>,
    source: &str,
    name: &str,
) -> Result<(), String> {
    let (start, end) = range(required(object, "range")?, name)?;
    if end > source.len() || !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return Err(format!("{name} must be a source UTF-8 range"));
    }
    Ok(())
}

fn validate_tab_stop(value: &Value) -> Result<(), String> {
    let stop = object(value, "tab stop")?;
    only_fields(stop, &["position", "alignment", "character"], "tab stop")?;
    non_negative_i32(stop.get("position"), "tab position")?;
    let alignment = enum_string(
        stop.get("alignment"),
        "tab alignment",
        &["start", "center", "end", "character"],
    )?;
    match (alignment, stop.get("character")) {
        ("character", Some(character)) => {
            one_char(Some(character), "tab character")?;
        },
        ("character", None) => return Err("character-aligned tab needs character".to_owned()),
        (_, Some(_)) => return Err("only a character-aligned tab accepts character".to_owned()),
        (_, None) => {},
    }
    Ok(())
}

fn validate_style(value: &Value) -> Result<(), String> {
    if value.is_string() {
        enum_string(Some(value), "style profile", PROFILES)?;
        return Ok(());
    }
    let style = object(value, "style")?;
    let names: BTreeSet<&str> = STYLE_CHOICES
        .iter()
        .map(|(name, _)| *name)
        .chain(std::iter::once("profile"))
        .collect();
    for (name, value) in style {
        if !names.contains(name.as_str()) {
            return Err(format!("unknown style setting {name:?}"));
        }
        if name == "profile" {
            enum_string(Some(value), "style profile", PROFILES)?;
            continue;
        }
        let choices = STYLE_CHOICES
            .iter()
            .find_map(|(candidate, choices)| (*candidate == name).then_some(*choices))
            .ok_or_else(|| format!("unknown style setting {name:?}"))?;
        enum_string(Some(value), name, choices)?;
    }
    Ok(())
}

fn validate_line(value: &Value) -> Result<(), String> {
    let line = object(value, "line")?;
    only_fields(
        line,
        &[
            "range",
            "inline_origin",
            "block_origin",
            "inline_extent",
            "block_extent",
            "clusters",
            "attachments",
        ],
        "line",
    )?;
    range(required(line, "range")?, "line range")?;
    i32_value(line.get("inline_origin"), "line inline_origin")?;
    i32_value(line.get("block_origin"), "line block_origin")?;
    non_negative_i32(line.get("inline_extent"), "line inline_extent")?;
    non_negative_i32(line.get("block_extent"), "line block_extent")?;
    for placement in array(required(line, "clusters")?, "line clusters")? {
        validate_placement(placement)?;
    }
    for attachment in array(required(line, "attachments")?, "line attachments")? {
        validate_attachment(attachment)?;
    }
    Ok(())
}

fn validate_placement(value: &Value) -> Result<(), String> {
    let placement = object(value, "cluster placement")?;
    only_fields(
        placement,
        &[
            "origin",
            "range",
            "inline",
            "block",
            "advance",
            "size",
            "frame",
            "writing_mode",
            "transform",
        ],
        "cluster placement",
    )?;
    let origin = object(required(placement, "origin")?, "placement origin")?;
    if origin.len() != 1 || !(origin.contains_key("cluster") || origin.contains_key("construct")) {
        return Err("placement origin needs exactly one cluster or construct ordinal".to_owned());
    }
    let ordinal = origin.get("cluster").or_else(|| origin.get("construct"));
    offset(ordinal, "placement origin ordinal")?;
    range(required(placement, "range")?, "placement range")?;
    i32_value(placement.get("inline"), "placement inline")?;
    i32_value(placement.get("block"), "placement block")?;
    non_negative_i32(placement.get("advance"), "placement advance")?;
    validate_size(required(placement, "size")?)?;
    frame(placement.get("frame"))?;
    writing_mode(placement.get("writing_mode"))?;
    transform(placement.get("transform"))?;
    Ok(())
}

fn validate_attachment(value: &Value) -> Result<(), String> {
    let attachment = object(value, "attachment")?;
    only_fields(
        attachment,
        &[
            "construct",
            "range",
            "inline",
            "block",
            "advance",
            "size",
            "writing_mode",
            "transform",
            "symbol",
        ],
        "attachment",
    )?;
    offset(attachment.get("construct"), "attachment construct ordinal")?;
    let symbol = required(attachment, "symbol")?;
    if symbol.is_null() {
        range(required(attachment, "range")?, "attachment range")?;
    } else {
        let (start, end) = offset_pair(required(attachment, "range")?, "attachment range")?;
        if start != end {
            return Err("a repeated-symbol attachment has an empty shaped range".to_owned());
        }
        one_char(Some(symbol), "attachment symbol")?;
    }
    i32_value(attachment.get("inline"), "attachment inline")?;
    i32_value(attachment.get("block"), "attachment block")?;
    non_negative_i32(attachment.get("advance"), "attachment advance")?;
    validate_size(required(attachment, "size")?)?;
    writing_mode(attachment.get("writing_mode"))?;
    transform(attachment.get("transform"))?;
    Ok(())
}

fn validate_diagnostic(value: &Value) -> Result<(), String> {
    let diagnostic = object(value, "diagnostic")?;
    only_fields(
        diagnostic,
        &["code", "severity", "range", "jlreq"],
        "diagnostic",
    )?;
    non_empty_string(diagnostic.get("code"), "diagnostic code")?;
    enum_string(
        diagnostic.get("severity"),
        "diagnostic severity",
        &["info", "warning", "error"],
    )?;
    match diagnostic.get("range") {
        Some(value) if value.is_null() => {},
        Some(value) => {
            range(value, "diagnostic range")?;
        },
        None => return Err("diagnostic range is required (and may be null)".to_owned()),
    }
    non_empty_string(diagnostic.get("jlreq"), "diagnostic jlreq reference")?;
    Ok(())
}

fn validate_size(value: &Value) -> Result<(), String> {
    let size = object(value, "size")?;
    only_fields(size, &["inline", "block"], "size")?;
    positive_i32(size.get("inline"), "size inline")?;
    positive_i32(size.get("block"), "size block")?;
    Ok(())
}

fn frame(value: Option<&Value>) -> Result<&str, String> {
    enum_string(value, "frame", &["full-em", "proportional", "half-em"])
}

fn writing_mode(value: Option<&Value>) -> Result<&str, String> {
    enum_string(value, "writing_mode", &["horizontal-tb", "vertical-rl"])
}

fn transform(value: Option<&Value>) -> Result<&str, String> {
    enum_string(
        value,
        "transform",
        &["identity", "rotate-clockwise", "tate-chu-yoko"],
    )
}

fn range(value: &Value, name: &str) -> Result<(usize, usize), String> {
    let (start, end) = offset_pair(value, name)?;
    if start >= end {
        return Err(format!("{name} must be non-empty and ordered"));
    }
    Ok((start, end))
}

fn offset_pair(value: &Value, name: &str) -> Result<(usize, usize), String> {
    let values = array(value, name)?;
    if values.len() != 2 {
        return Err(format!("{name} must contain exactly two offsets"));
    }
    let start = offset(values.first(), &format!("{name} start"))?;
    let end = offset(values.get(1), &format!("{name} end"))?;
    Ok((start, end))
}

fn only_fields(object: &Map<String, Value>, allowed: &[&str], name: &str) -> Result<(), String> {
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(format!("unknown {name} field {field:?}"));
    }
    Ok(())
}

fn required<'a>(object: &'a Map<String, Value>, name: &str) -> Result<&'a Value, String> {
    object
        .get(name)
        .ok_or_else(|| format!("{name} is required"))
}

fn object<'a>(value: &'a Value, name: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{name} must be an object"))
}

fn array<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{name} must be an array"))
}

fn string<'a>(value: Option<&'a Value>, name: &str) -> Result<&'a str, String> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{name} must be a string"))
}

fn non_empty_string<'a>(value: Option<&'a Value>, name: &str) -> Result<&'a str, String> {
    let value = string(value, name)?;
    if value.is_empty() {
        Err(format!("{name} must not be empty"))
    } else {
        Ok(value)
    }
}

fn enum_string<'a>(
    value: Option<&'a Value>,
    name: &str,
    choices: &[&str],
) -> Result<&'a str, String> {
    let value = string(value, name)?;
    if choices.contains(&value) {
        Ok(value)
    } else {
        Err(format!("unknown {name} {value:?}"))
    }
}

fn i32_value(value: Option<&Value>, name: &str) -> Result<i32, String> {
    let value = value
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{name} must be an integer"))?;
    i32::try_from(value).map_err(|_| format!("{name} is outside i32"))
}

fn non_negative_i32(value: Option<&Value>, name: &str) -> Result<i32, String> {
    let value = i32_value(value, name)?;
    if value < 0 {
        Err(format!("{name} must not be negative"))
    } else {
        Ok(value)
    }
}

fn positive_i32(value: Option<&Value>, name: &str) -> Result<i32, String> {
    let value = i32_value(value, name)?;
    if value <= 0 {
        Err(format!("{name} must be positive"))
    } else {
        Ok(value)
    }
}

fn positive_u16(value: Option<&Value>, name: &str) -> Result<u16, String> {
    let value = value
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(|| format!("{name} must be a u16 integer"))?;
    if value == 0 {
        Err(format!("{name} must be positive"))
    } else {
        Ok(value)
    }
}

fn offset(value: Option<&Value>, name: &str) -> Result<usize, String> {
    value
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| format!("{name} must be a non-negative platform-sized integer"))
}

fn one_char(value: Option<&Value>, name: &str) -> Result<char, String> {
    let value = string(value, name)?;
    let mut characters = value.chars();
    let character = characters
        .next()
        .ok_or_else(|| format!("{name} must not be empty"))?;
    if characters.next().is_some() {
        Err(format!("{name} must contain one Unicode scalar"))
    } else {
        Ok(character)
    }
}
