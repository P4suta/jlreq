// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io::BufRead;

/// Read one bounded NDJSON line into reusable storage.
///
/// The returned boolean is false only when the reader reaches EOF before
/// producing another line. The byte counters include a consumed newline, as
/// required by the conformance protocol's stream limits.
pub(crate) fn read_limited_line(
    reader: &mut dyn BufRead,
    line: &mut Vec<u8>,
    max_message_bytes: usize,
    max_total_bytes: usize,
    total: &mut usize,
    stream_name: &str,
) -> Result<bool, String> {
    line.clear();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| format!("could not read {stream_name}: {error}"))?;
        if available.is_empty() {
            return Ok(!line.is_empty());
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let content = newline.unwrap_or(available.len());
        let consumed = content.saturating_add(usize::from(newline.is_some()));
        *total = total.saturating_add(consumed);
        if *total > max_total_bytes {
            return Err(format!(
                "{stream_name} exceeds the {max_total_bytes} byte total limit"
            ));
        }
        if line.len().saturating_add(content) > max_message_bytes {
            return Err(format!(
                "{stream_name} message exceeds the {max_message_bytes} byte line limit"
            ));
        }
        line.extend_from_slice(&available[..content]);
        reader.consume(consumed);
        if newline.is_some() {
            return Ok(true);
        }
    }
}
