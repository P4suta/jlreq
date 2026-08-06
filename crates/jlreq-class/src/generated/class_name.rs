// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The thirty classes of §3.9.2: the id, the name in both locales, and the Appendix A section that enumerates each.
//!
//! Do not edit. `cargo run -p xtask -- generate` writes this file, and
//! `generate --check` fails when regenerating it would change a byte. A hand
//! edit is a bug even when it is correct, because the next revision of the
//! specification will not carry it forward (ADR 0009).
//!
//! - Source: `spec/derived/classes.tsv`
//! - Source SHA-256: `f19d929b3880d94fdbb6ccc7c6b60e98bd1523a7c830b5f3cdeebb5db6f495a9`
//! - Specification: JLReq, 2020-08-11
//! - Generator: `xtask/src/classes.rs`, `xtask/src/generate.rs`
//! - Generator SHA-256: `d956a3e7a92bc9dc3250848ed8a86db702f88b172a36826b7c9ff89d8eb6930f`
//! - Entries: 30

/// One character class, as §3.9.2 names it.
///
/// JLReq: §3.9.2
#[derive(Debug)]
pub(crate) struct ClassName {
    /// The id the published document anchors this class by: `cl-01` … `cl-30`,
    /// which is also the identifier every rule sentence of JLReq uses.
    pub(crate) id: &'static str,
    /// The English name, as §3.9.2's own list writes it.
    pub(crate) en: &'static str,
    /// The Japanese name, likewise.
    pub(crate) ja: &'static str,
    /// The canonical address of the Appendix A section enumerating this class,
    /// or the empty string for the five that enumerate nothing.
    pub(crate) enumeration: &'static str,
}

/// The thirty classes §3.9.2 closes the set at, in class-number order.
///
/// JLReq: §3.9.2, §A
pub(crate) const CLASSES: &[ClassName] = &[
    ClassName {
        id: "cl-01",
        en: "Opening brackets",
        ja: "始め括弧類",
        enumeration: "A.1",
    },
    ClassName {
        id: "cl-02",
        en: "Closing brackets",
        ja: "終わり括弧類",
        enumeration: "A.2",
    },
    ClassName {
        id: "cl-03",
        en: "Hyphens",
        ja: "ハイフン類",
        enumeration: "A.3",
    },
    ClassName {
        id: "cl-04",
        en: "Dividing punctuation marks",
        ja: "区切り約物",
        enumeration: "A.4",
    },
    ClassName {
        id: "cl-05",
        en: "Middle dots",
        ja: "中点類",
        enumeration: "A.5",
    },
    ClassName {
        id: "cl-06",
        en: "Full stops",
        ja: "句点類",
        enumeration: "A.6",
    },
    ClassName {
        id: "cl-07",
        en: "Commas",
        ja: "読点類",
        enumeration: "A.7",
    },
    ClassName {
        id: "cl-08",
        en: "Inseparable characters",
        ja: "分離禁止文字",
        enumeration: "A.8",
    },
    ClassName {
        id: "cl-09",
        en: "Iteration marks",
        ja: "繰返し記号",
        enumeration: "A.9",
    },
    ClassName {
        id: "cl-10",
        en: "Prolonged sound marks",
        ja: "長音記号",
        enumeration: "A.10",
    },
    ClassName {
        id: "cl-11",
        en: "Small kana",
        ja: "小書きの仮名",
        enumeration: "A.11",
    },
    ClassName {
        id: "cl-12",
        en: "Prefixed abbreviations",
        ja: "前置省略記号",
        enumeration: "A.12",
    },
    ClassName {
        id: "cl-13",
        en: "Postfixed abbreviations",
        ja: "後置省略記号",
        enumeration: "A.13",
    },
    ClassName {
        id: "cl-14",
        en: "Full-width ideographic space",
        ja: "和字間隔",
        enumeration: "A.14",
    },
    ClassName {
        id: "cl-15",
        en: "Hiragana",
        ja: "平仮名",
        enumeration: "A.15",
    },
    ClassName {
        id: "cl-16",
        en: "Katakana",
        ja: "片仮名",
        enumeration: "A.16",
    },
    ClassName {
        id: "cl-17",
        en: "Math symbols",
        ja: "等号類",
        enumeration: "A.17",
    },
    ClassName {
        id: "cl-18",
        en: "Math operators",
        ja: "演算記号",
        enumeration: "A.18",
    },
    ClassName {
        id: "cl-19",
        en: "Ideographic characters",
        ja: "漢字等",
        enumeration: "A.19",
    },
    ClassName {
        id: "cl-20",
        en: "Characters as reference marks",
        ja: "合印中の文字",
        enumeration: "",
    },
    ClassName {
        id: "cl-21",
        en: "Ornamented character complexes",
        ja: "親文字群中の文字（添え字付き）",
        enumeration: "",
    },
    ClassName {
        id: "cl-22",
        en: "Simple-ruby character complexes",
        ja: "親文字群中の文字（熟語ルビ以外のルビ付き）",
        enumeration: "",
    },
    ClassName {
        id: "cl-23",
        en: "Jukugo-ruby character complexes",
        ja: "親文字群中の文字（熟語ルビ付き）",
        enumeration: "",
    },
    ClassName {
        id: "cl-24",
        en: "Grouped numerals",
        ja: "連数字中の文字",
        enumeration: "A.24",
    },
    ClassName {
        id: "cl-25",
        en: "Unit symbols",
        ja: "単位記号中の文字",
        enumeration: "A.25",
    },
    ClassName {
        id: "cl-26",
        en: "Western word space",
        ja: "欧文間隔",
        enumeration: "A.26",
    },
    ClassName {
        id: "cl-27",
        en: "Western characters",
        ja: "欧文用文字",
        enumeration: "A.27",
    },
    ClassName {
        id: "cl-28",
        en: "Warichu opening brackets",
        ja: "割注始め括弧類",
        enumeration: "A.28",
    },
    ClassName {
        id: "cl-29",
        en: "Warichu closing brackets",
        ja: "割注終わり括弧類",
        enumeration: "A.29",
    },
    ClassName {
        id: "cl-30",
        en: "Characters in tate-chu-yoko",
        ja: "縦中横中の文字",
        enumeration: "",
    },
];
