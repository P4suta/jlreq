// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The reader, over the subset of JSON the case format uses.
//!
//! `docs/design/api-spine.md` makes this crate's freedom from outside dependencies a
//! decision rather than an omission: the suite is the deliverable [ADR
//! 0006](https://github.com/P4suta/kumihan/blob/main/docs/adr/0006-conformance-suite-as-artifact.md)
//! says is worth more than the implementation, and a browser engineer running it should not
//! acquire a proc-macro chain to do so. The parser is unusually safe to own here because
//! [ADR
//! 0005](https://github.com/P4suta/kumihan/blob/main/docs/adr/0005-integer-layout-units.md)
//! already guarantees that every number in a case is an integer inside 2^53 — the one part
//! of JSON that is genuinely hard is the part this format does not contain.
//!
//! A number is therefore an integer here, and a fraction or an exponent is a reading error
//! naming that guarantee rather than a value this type can hold. The committed schema stays
//! committed, so nobody else has to use this reader.

use core::fmt;

/// The largest integer an IEEE-754 double holds exactly.
const EXACT_INTEGER_CEILING: i64 = 1 << 53;

/// The same bound below zero.
const EXACT_INTEGER_FLOOR: i64 = -(1 << 53);

/// How deep a case file may nest. Far above anything the format needs; it exists so a
/// malformed file cannot recurse this reader past the stack.
const MAX_DEPTH: u8 = 32;

/// A JSON value, over the subset the case format uses.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Json {
    /// `null`.
    Nothing,
    /// `true` or `false`.
    Truth(bool),
    /// An integer inside 2^53.
    Integer(i64),
    /// A string.
    Text(String),
    /// An array.
    Array(Vec<Json>),
    /// An object, in the order the file writes it, with no repeated name.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// Read one value, and nothing after it.
    pub fn parse(source: &str) -> Result<Self, JsonError> {
        let mut reader = Reader {
            bytes: source.as_bytes(),
            at: 0,
            line: 1,
        };
        let value = reader.value(0)?;
        reader.skip_space();
        if reader.peek().is_some() {
            return Err(reader.fault("a case file holds one object"));
        }
        Ok(value)
    }

    /// The value under `name`, when this is an object that has one.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Self> {
        self.as_object()?
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// The members, when this is an object.
    #[must_use]
    pub fn as_object(&self) -> Option<&[(String, Self)]> {
        match self {
            Self::Object(members) => Some(members),
            _ => None,
        }
    }

    /// The entries, when this is an array.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(entries) => Some(entries),
            _ => None,
        }
    }

    /// The string, when this is one.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The integer, when this is one.
    #[must_use]
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// The boolean, when this is one.
    #[must_use]
    pub fn as_truth(&self) -> Option<bool> {
        match self {
            Self::Truth(value) => Some(*value),
            _ => None,
        }
    }
}

/// Why a file is not the JSON this format accepts, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct JsonError {
    /// The line the reader stopped on, counted from one.
    line: usize,
    /// What was wrong there.
    reason: String,
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "line {line}: {reason}",
            line = self.line,
            reason = self.reason
        )
    }
}

/// The hand-rolled reader over the subset the case format uses.
#[derive(Debug)]
struct Reader<'a> {
    /// The whole file.
    bytes: &'a [u8],
    /// How far the reader has come.
    at: usize,
    /// Which line that is, counted from one.
    line: usize,
}

impl Reader<'_> {
    /// The byte under the cursor.
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// Step over one byte, counting lines.
    fn bump(&mut self) {
        if self.peek() == Some(b'\n') {
            self.line = self.line.saturating_add(1);
        }
        self.at = self.at.saturating_add(1);
    }

    /// Step over the whitespace JSON allows between values.
    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.bump();
        }
    }

    /// A reading error at the cursor.
    fn fault(&self, reason: &str) -> JsonError {
        JsonError {
            line: self.line,
            reason: reason.to_owned(),
        }
    }

    /// Read one value.
    fn value(&mut self, depth: u8) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.fault("nests deeper than a case ever does"));
        }
        self.skip_space();
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.text().map(Json::Text),
            Some(b't') => self.word("true", Json::Truth(true)),
            Some(b'f') => self.word("false", Json::Truth(false)),
            Some(b'n') => self.word("null", Json::Nothing),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.fault("expected a value")),
        }
    }

    /// Read one of the three bare words.
    fn word(&mut self, word: &str, value: Json) -> Result<Json, JsonError> {
        let end = self.at.saturating_add(word.len());
        if self.bytes.get(self.at..end) != Some(word.as_bytes()) {
            return Err(self.fault("expected a value"));
        }
        for _ in 0..word.len() {
            self.bump();
        }
        Ok(value)
    }

    /// Read an object, rejecting a repeated name.
    fn object(&mut self, depth: u8) -> Result<Json, JsonError> {
        self.bump();
        let mut members: Vec<(String, Json)> = Vec::new();
        self.skip_space();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_space();
            let name = self.text()?;
            if members.iter().any(|(key, _)| *key == name) {
                return Err(self.fault(&format!("names `{name}` twice")));
            }
            self.skip_space();
            if self.peek() != Some(b':') {
                return Err(self.fault("expected `:` after a name"));
            }
            self.bump();
            let value = self.value(depth.saturating_add(1))?;
            members.push((name, value));
            self.skip_space();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b'}') => {
                    self.bump();
                    return Ok(Json::Object(members));
                },
                _ => return Err(self.fault("expected `,` or `}`")),
            }
        }
    }

    /// Read an array.
    fn array(&mut self, depth: u8) -> Result<Json, JsonError> {
        self.bump();
        let mut entries = Vec::new();
        self.skip_space();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(Json::Array(entries));
        }
        loop {
            entries.push(self.value(depth.saturating_add(1))?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b']') => {
                    self.bump();
                    return Ok(Json::Array(entries));
                },
                _ => return Err(self.fault("expected `,` or `]`")),
            }
        }
    }

    /// Read a string, decoding the escapes JSON defines.
    fn text(&mut self) -> Result<String, JsonError> {
        if self.peek() != Some(b'"') {
            return Err(self.fault("expected a string"));
        }
        self.bump();
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None => return Err(self.fault("a string is not closed")),
                Some(b'"') => {
                    self.bump();
                    return String::from_utf8(out).map_err(|_| self.fault("is not UTF-8"));
                },
                Some(b'\\') => {
                    self.bump();
                    self.escape(&mut out)?;
                },
                Some(byte) if byte < 0x20 => {
                    return Err(self.fault("a string holds a raw control character"));
                },
                Some(byte) => {
                    out.push(byte);
                    self.bump();
                },
            }
        }
    }

    /// Decode one escape, including a surrogate pair.
    fn escape(&mut self, out: &mut Vec<u8>) -> Result<(), JsonError> {
        let escape = self
            .peek()
            .ok_or_else(|| self.fault("an escape is cut off"))?;
        self.bump();
        let plain = match escape {
            b'"' => Some(b'"'),
            b'\\' => Some(b'\\'),
            b'/' => Some(b'/'),
            b'b' => Some(0x08),
            b'f' => Some(0x0C),
            b'n' => Some(b'\n'),
            b'r' => Some(b'\r'),
            b't' => Some(b'\t'),
            _ => None,
        };
        if let Some(byte) = plain {
            out.push(byte);
            return Ok(());
        }
        if escape != b'u' {
            return Err(self.fault("is not an escape JSON defines"));
        }
        let point = self.code_point()?;
        let mut buffer = [0_u8; 4];
        out.extend_from_slice(point.encode_utf8(&mut buffer).as_bytes());
        Ok(())
    }

    /// Read one `\u` escape, pairing surrogates.
    fn code_point(&mut self) -> Result<char, JsonError> {
        let first = self.hex4()?;
        if !(0xD800..0xDC00).contains(&first) {
            return char::from_u32(first).ok_or_else(|| self.fault("is not a code point"));
        }
        if self.peek() != Some(b'\\') {
            return Err(self.fault("a leading surrogate is unpaired"));
        }
        self.bump();
        if self.peek() != Some(b'u') {
            return Err(self.fault("a leading surrogate is unpaired"));
        }
        self.bump();
        let second = self.hex4()?;
        if !(0xDC00..0xE000).contains(&second) {
            return Err(self.fault("a leading surrogate is unpaired"));
        }
        let high = first
            .checked_sub(0xD800)
            .and_then(|part| part.checked_mul(0x400));
        let point = high
            .and_then(|high| high.checked_add(second.wrapping_sub(0xDC00)))
            .and_then(|part| part.checked_add(0x1_0000));
        point
            .and_then(char::from_u32)
            .ok_or_else(|| self.fault("is not a code point"))
    }

    /// Read four hexadecimal digits.
    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut value: u32 = 0;
        for _ in 0..4_u8 {
            let digit = self
                .peek()
                .and_then(|byte| char::from(byte).to_digit(16))
                .ok_or_else(|| self.fault("an escape needs four hexadecimal digits"))?;
            value = value
                .checked_mul(16)
                .and_then(|shifted| shifted.checked_add(digit))
                .ok_or_else(|| self.fault("an escape needs four hexadecimal digits"))?;
            self.bump();
        }
        Ok(value)
    }

    /// Read a number, which this format guarantees is an integer inside 2^53.
    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        let digits = self.at;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump();
        }
        if self.at == digits {
            return Err(self.fault("expected a number"));
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(self.fault(
                "states a fraction or an exponent; every number in a case is an integer, \
                 which is what lets a case be compared exactly (ADR 0005)",
            ));
        }
        let literal = self
            .bytes
            .get(start..self.at)
            .and_then(|bytes| core::str::from_utf8(bytes).ok())
            .ok_or_else(|| self.fault("expected a number"))?;
        self.finish_number(literal)
    }

    /// Turn a number's literal text into a value, holding it to the format's guarantees.
    fn finish_number(&self, literal: &str) -> Result<Json, JsonError> {
        let digits = literal.strip_prefix('-').unwrap_or(literal);
        if digits.len() > 1 && digits.starts_with('0') {
            return Err(self.fault("states a number with a leading zero"));
        }
        let value: i64 = literal
            .parse()
            .map_err(|_| self.fault("states a number outside 2^53"))?;
        if !(EXACT_INTEGER_FLOOR..=EXACT_INTEGER_CEILING).contains(&value) {
            return Err(self.fault(
                "states a number outside 2^53, which a harness reading the case with \
                 doubles would not hold exactly",
            ));
        }
        Ok(Json::Integer(value))
    }
}

#[cfg(test)]
mod tests {
    use super::Json;

    #[test]
    fn an_object_reads_in_the_order_the_file_writes_it() {
        let value = Json::parse("{ \"b\": 1, \"a\": [true, null, \"x\"] }").expect("well formed");
        let members = value.as_object().expect("an object");
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].0, "b");
        assert_eq!(value.get("b").and_then(Json::as_integer), Some(1));
        assert_eq!(
            value
                .get("a")
                .and_then(Json::as_array)
                .and_then(|entries| entries.first())
                .and_then(Json::as_truth),
            Some(true)
        );
    }

    #[test]
    fn a_fraction_is_a_reading_error_naming_the_guarantee_it_breaks() {
        let fault = Json::parse("{ \"a\": 1.5 }").expect_err("ADR 0005 forbids it");
        assert!(fault.to_string().contains("integer"), "{fault}");
    }

    #[test]
    fn a_repeated_name_is_refused_rather_than_taken_twice() {
        assert!(Json::parse("{ \"a\": 1, \"a\": 2 }").is_err());
    }

    #[test]
    fn an_escape_decodes_including_a_surrogate_pair() {
        let value = Json::parse("\"\\u3002\\uD83D\\uDE00\\n\"").expect("well formed");
        assert_eq!(value.as_text(), Some("\u{3002}\u{1F600}\n"));
    }
}
