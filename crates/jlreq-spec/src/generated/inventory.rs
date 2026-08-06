// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The rule inventory: one row and one named identifier per statement of JLReq.
//!
//! Do not edit. `cargo run -p xtask -- generate` writes this file, and
//! `generate --check` fails when regenerating it would change a byte. A hand
//! edit is a bug even when it is correct, because the next revision of the
//! specification will not carry it forward (ADR 0009).
//!
//! - Source: `spec/derived/rules.tsv`
//! - Source SHA-256: `e6cc202eba14b2da71043f4968011e5f5152fdb4785ab2a84c719c939a62d7cf`
//! - Specification: JLReq, 2020-08-11
//! - Generator: `xtask/src/generate.rs`, `xtask/src/inventory.rs`
//! - Generator SHA-256: `cd0d19a664846f3c4cfd9f627e77114ff724b52bede4bb8403117e60c47d3381`
//! - Entries: 106

use crate::rule::{Address, Appendix, Rule, RuleId, Standing};

/// Every inventoried rule, in the specification's own reading order.
///
/// The address is written as its components rather than as text, because
/// a table read at run time is not a `const`; `crates/jlreq-spec/src/
/// rule.rs` reads every one of them back through the address grammar at
/// compile time, so a component the grammar refuses is a build failure
/// rather than a rule nobody can cite.
///
/// JLReq: §3, §B, §C, §D, §E, §F
pub(crate) const RULES: &[Rule] = &[
    Rule {
        address: Address::assembled(None, [3, 1, 1, 0], 3, 0),
        statement: "There are some punctuation marks that are used uniquely in either vertical writing mode or horizontal writing mode. In this document, characters and symbols are treated as members of a character class, classified by their behavior for composition. Each class name is followed by class id, such as opening brackets (cl-01). Details are explained in § 3.9 About Character Classes . The following are some typical examples:",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 2, 0], 3, 0),
        statement: "The positioning of punctuation marks (commas, periods and brackets) in a line proceeds as follows.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 3, 0], 3, 0),
        statement: "The spacing usually added after IDEOGRAPHIC COMMA \"、\" and the spacing before and after KATAKANA MIDDLE DOT \"・\" are omitted, in principle, for cosmetic reasons in the following cases.",
        direction_conditional: true,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 4, 0], 3, 0),
        statement: "In cases where multiple punctuation marks, such as opening brackets (cl-01), closing brackets (cl-02), commas (cl-07), full stops (cl-06) and middle dots (cl-05), come one after the other, the following spacing adjustments are made for aesthetic reasons (see Figure 69). Note also that the half em and quarter em spacing added before or after punctuation marks, including the half em spacing after full stops (cl-06) appearing in the middle of a line, are subject, in principle, to line adjustment and may eventually be removed, except for those added after full stops (cl-06). (See § 3.8 Line Adjustment for more about line adjustment.) For more information about the positioning of closing brackets (cl-02), full stops (cl-06), commas (cl-07) and middle dots (cl-05) at line end, see § 3.1.9 Positioning of Closing Brackets, Full Stops, Commas and Middle Dots at Line End .",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 5, 0], 3, 0),
        statement: "When starting a new line with opening brackets (cl-01) there are some patterns as shown in Figure 71. Note that the amount of line indent after the line feed (the first line indent of a new paragraph) is assumed to be a one em space across all the patterns.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 6, 0], 3, 0),
        statement: "The dividing punctuation marks (cl-04) (QUESTION MARK \"?\" and EXCLAMATION MARK \"!\") should be full-width, and they are typeset as follows.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 7, 0], 3, 0),
        statement: "In principle, no line should begin with closing brackets (cl-02), hyphens (cl-03), dividing punctuation marks (cl-04), middle dots (cl-05), full stops (cl-06), commas (cl-07), iteration marks (cl-09), a prolonged sound mark (cl-10), small kana (cl-11) or warichu closing brackets (cl-29) (line-start prohibition rule). Otherwise the line would have an odd appearance.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 8, 0], 3, 0),
        statement: "No line should end with opening brackets (cl-01) or warichu opening brackets (cl-28) (line-end prohibition rules). Otherwise the line would have an odd appearance.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 9, 0], 3, 0),
        statement: "In principle, closing brackets (cl-02), commas (cl-07) or full stops (cl-06) at the line end have half em spacing after them (see Figure 76). This half em spacing can be removed for line adjustment (for more about line adjustment, see § 3.8 Line Adjustment ). However, the possibilities are only half em spacing or solid. Other spacing, such as quarter em spacing should not be used. In principle, the middle dot (cl-05) character at the line end also has quarter em spacing before and after, and is handled like a full-width character (see Figure 76). This quarter em spacing can also be removed for line adjustment, namely middle dots (cl-05) can be set solid before and after (about line adjustment, see § 3.8 Line Adjustment ). However, in this case also, the only possibilities are quarter em spacing or solid setting. Other intermediate-sized spacing should not be used.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 10, 0], 3, 0),
        statement: "If the following characters and symbols appear in sequence there will be no line break between them. The reason is that these characters and symbols are to be handled as one unit.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 11, 0], 3, 0),
        statement: "For line adjustment processing, spacing must not be increased between the following characters. (This is called the inseparable characters rule.) The reason is that these characters or symbols should appear as one unit (for more about line adjustment, see § 3.8 Line Adjustment ).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 1, 12, 0], 3, 0),
        statement: "Methods of line adjustment processing are discussed in § 3.8 Line Adjustment . However, since layout processing of punctuation marks is one reason for the need for line adjustment processing, we will here introduce two main examples of cases where line adjustment processing is necessary, and show adjustment examples (see Figure 89).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 2, 1, 0], 3, 0),
        statement: "There are a lot of examples of Japanese text in which Western and/or Greek letters are mixed among Japanese letters. Examples are as follows:",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 2, 2, 0], 3, 0),
        statement: "In horizontal writing mode the basic approach is to use proportional Western fonts (Figure 90). For European numerals, both half-width fonts and proportional fonts are used. Note that Western word space (cl-26) is a one third em space, in principle, except at line head, line head of warichu, line end and line end of warichu. Western word space (cl-26) at line head, line head of warichu, line end and line end of warichu, is set solid.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 2, 3, 0], 3, 0),
        statement: "As explained in § 2.3.2 Major Differences between Vertical Writing Mode and Horizontal Writing Mode , there are three different styles for setting Latin letters and European numerals in vertical writing mode:",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 2, 4, 0], 3, 0),
        statement: "When full-width and fixed-width Western characters or European numerals are set in vertical writing mode as \"quasi\" Japanese characters, inter-character spacing between these characters and hiragana (cl-15), katakana (cl-16) or ideographic characters (cl-19) are set solid, similar to ordinary ideographic characters (cl-19) (see Figure 99). Also, in principle, when full-width and fixed-width Western characters or European numerals are set after full stops (cl-06), commas (cl-07) or closing brackets (cl-02), or before opening brackets (cl-01), insert half em spacing after commas (cl-07) or closing brackets (cl-02), or before opening brackets (cl-01). In addition, in these cases, insert half em spacing after full stops (cl-06). When full-width and fixed-width Western characters or European numerals are set before a full stop (cl-06), comma (cl-07) or closing bracket (cl-02), or after an opening bracket (cl-01), the inter-character spacing before the full stop, comma or closing bracket, or after the opening bracket is set solid.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 2, 5, 0], 3, 0),
        statement: "To set strings as tate-chu-yoko (horizontal-in-vertical setting), first set from left to right using solid setting, then align the whole string to the center of the vertical line (Figure 101). When hiragana (cl-15), katakana (cl-16) or ideographic characters (cl-19) are set before/after tate-chu-yoko, the inter-character spacing is set solid. In principle, when tate-chu-yoko is set after a comma (cl-07) or closing bracket (cl-02), or before an opening bracket (cl-01), half em spacing is added. In addition, when tate-chu-yoko is set after a full stop (cl-06) in the middle of a line, half em spacing is added. When a full stop (cl-06) is set at the end of a line, half em spacing is inserted after it, in principle. When tate-chu-yoko is set before full stops, commas or closing brackets, or after opening brackets, the inter-character spacing is set solid.",
        direction_conditional: true,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 2, 6, 0], 3, 0),
        statement: "Composition rules for Western characters, Western text and European numerals, set rotated 90 degrees clockwise in vertical writing mode, and horizontal writing mode, are as follows:",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 3, 1, 0], 3, 0),
        statement: "Ruby is a small-sized, supplementary text attached to a character or a group of characters in the main text. A run of ruby text, usually attached to the right of the characters in vertical writing mode or immediately above them in horizontal writing mode, indicates the reading or the meaning of those characters (see Figure 105). The characters in the main text that are annotated by ruby are called \"base characters\". Mainly Hiragana (cl-15) characters are often used for ruby to indicate how to read ideographic characters (cl-19); this is known as ruby annotation or as \"furigana\".",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 3, 2, 0], 3, 0),
        statement: "There are several methods of choosing how to attach ruby annotations to which base characters.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 3, 3, 0], 3, 0),
        statement: "The character size of ruby characters is, in principle, the half size of the base characters (see Figure 114).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 3, 4, 0], 3, 0),
        statement: "In principle, ruby is attached to the right of base characters in vertical writing mode, and above in horizontal writing mode.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 3, 5, 0], 3, 0),
        statement: "When mono-ruby characters are Japanese, they are set solid. If mono-ruby characters have their own character widths such as Western characters or European numerals, they are set according to their own widths and then the ruby text is placed so that its center matches that of its base character. There are more variations depending on the combination of the base character and ruby text and accordingly various composition rules have been invented, which will be explained with examples.",
        direction_conditional: true,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 3, 6, 0], 3, 0),
        statement: "When the length of a sequence of base characters (number of characters * advance-width of each character) and that of the ruby text are the same, each text is set solid and the center of both texts are aligned with each other (see Figure 123).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 3, 7, 0], 3, 0),
        statement: "If the number of ruby characters are two or less for each ideographic characters (cl-19) which participates in a kanji compound word (or jukugo), then for each run of ruby text associated with each base character, compose ruby characters as described in § 3.3.5 Positioning of Mono-ruby with Respect to Base Characters (see Figure 129).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 3, 8, 0], 3, 0),
        statement: "When the length of any ruby text is shorter than that of the base characters, the main text can be just set solid because there is no need for any adjustment of the inter-character spacing between base characters and their adjacent characters in the main text.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 3, 9, 0], 3, 0),
        statement: "Emphasis dots (also known as bouten or side dots) are symbols placed alongside a run of ideographic character (cl-19) or hiragana (cl-15) characters to emphasize the text.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 4, 1, 0], 3, 0),
        statement: "Warichu (inline cutting note) is a type of inline notation, where two lines of small characters are inserted into the text. Warichu divides a line into two sub lines. The frequency of use of the inline cutting note is not so high. However, the inline cutting note is very important for study guides, travel guides, reference books, encyclopedias and manuals, because it is very effective for inserting notes at the point in the text where they are needed (see Figure 145). Inline cutting note is usually used in vertical writing mode. It is very infrequently used in horizontal writing mode.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 4, 2, 0], 3, 0),
        statement: "Character size for an inline cutting note depends on the character size established for the kihon-hanmen. Usually, around six point size is used (see Figure 145).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 4, 3, 0], 3, 0),
        statement: "When an inline cutting note will not fit on a single kihon-hanmen line, it will wrap onto the following line, and will be set as shown in Figure 148 or Figure 149.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 5, 1, 0], 3, 0),
        statement: "A paragraph, a section of a document which consists of one or more sentences to indicate a distinct idea, usually begins on a new line. For the related line head indent at the beginning of paragraphs (in JIS 4051, this is called the \"paragraph line head indent\") the following methods are available. The amount of spacing used for the indentation is, in principle, one em spacing using the character size in the paragraph.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 5, 2, 0], 3, 0),
        statement: "The line head indent is the indentation of the line head by a fixed amount, starting from the line head side of the hanmen (in the case of one column) or of the column area (in the case of several columns). In contrast, the indentation of the line end position by a fixed amount, starting from the line head, is called line end indent.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 5, 3, 0], 3, 0),
        statement: "The Japanese \"single line alignment method\" is a process for setting alignment for a run of text that is shorter than a given line length. This method is frequently used for headings and poems. The following methods are available (see Figure 157).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 5, 4, 0], 3, 0),
        statement: "The intent of widow adjustment of paragraphs is to avoid that the last line of a paragraph contains less than a given number of characters. This is also called \"widow\" processing.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 6, 1, 0], 3, 0),
        statement: "Tab setting is useful for alignment of table data, itemized lists, etc. where a series of characters need to be set at specific alignment positions within a line (see Figure 160).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 6, 2, 0], 3, 0),
        statement: "There are the following types of tab setting to align texts.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 6, 3, 0], 3, 0),
        statement: "Set the text from the line head to the position before the tab sign in the first tab position, set the text from the first tab sign to the next tab sign in the second tab position, and so on. The behavior of opening brackets (cl-01) and closing brackets (cl-02), etc. is same as for the main text.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 7, 1, 0], 3, 0),
        statement: "Superscripts and subscripts are small letters associated with base characters, and typically used to indicate SI unit symbols, or used for mathematical or chemical formulae.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 7, 2, 0], 3, 0),
        statement: "Furiwake is a typesetting style for setting multiple phrases or sentences in the middle of a line. Furiwake is also used to indicate options (see Figure 171). Study guides, manuals and reference books sometimes use furiwake. In many furiwake styles, multiple lines are indicated with opening brackets (cl-01) and closing brackets (cl-02), etc.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 7, 3, 0], 3, 0),
        statement: "In cases such as lists of names of Japanese people, the length of some part of the text may be explicitly defined. In such cases, different numbers of characters are set, using adjustment of the inter-character spacing, so that they are all aligned to the same length. This is called jidori processing (see Figure 173).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 7, 4, 0], 3, 0),
        statement: "Math symbols and math operators, such as EQUALS SIGN \"=\", APPROXIMATELY EQUAL TO OR THE IMAGE OF \"≒\", PLUS SIGN \"+\" and MINUS SIGN \"−\" are commonly used not only for scientific and technical documents but also for ordinary books. In the Japanese composition system, there are two different groups of math symbols, which are each treated differently. So in this document math symbols are classified into two different classes; math symbols (cl-17) and math operators (cl-18).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 8, 1, 0], 3, 0),
        statement: "Line adjustment processing is applied where inter-character adjustments are needed to bring the line end into the correct alignment, e.g. because of line wrap or other reasons. Within a paragraph, lines are created by separating character sequences at places where line breaking is not prohibited. Except for the end of the last line of a paragraph, it is necessary to set the head and end of each line at predicable, aligned positions. For the last line of the paragraph, it is still necessary to set the head at the aligned position, however the line end need not aligned to the other alignment position. To achieve this, only inter-character spacing indicated in the table of § B. Spacing between Characters , or explicitly chosen spacing, are added, and other inter-character spacing is set solid.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 8, 2, 0], 3, 0),
        statement: "Line adjustment processing targets places with a predefined spacing or solid setting. Methods for line adjustment are as follows.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 8, 3, 0], 3, 0),
        statement: "For line adjustment by inter-character spacing reduction decisions must first be made about the preferred order in which reduction processing options are applied, and the maximum amount of spacing reduction needed. Inter-character spacing reduction is processed with following priorities.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 8, 4, 0], 3, 0),
        statement: "As with line adjustment by inter-character spacing reduction, for line adjustment by inter-character spacing expansion at first the order of processing and the maximum amount of spacing to be added are defined. In JIS X 4051, the following processing order is defined.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 9, 1, 0], 3, 0),
        statement: "The positioning of characters and symbols may vary depending on the following.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 9, 2, 0], 3, 0),
        statement: "During layout processing, the issues mentioned in the previous section are addressed by grouping characters and symbols according to their characteristics, and handling them as character classes.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(None, [3, 9, 3, 0], 3, 0),
        statement: "For each character class it is possible to describe whether the characters may appear at the line head or line end or not, the positioning method for the line head or line end positions (if available), the amount of spacing between sequences of several characters, and the combination with character classes before or after the characters (in a 2 dimensional table). In JIS X 4051 this is shown in table 5 \"Amount of spacing (between characters)\".",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [0, 0, 0, 0], 0, 0),
        statement: "The amount of spacing between two adjacent characters of given character classes explained in § 3.9.2 Grouping of Characters and Symbols depending on their Positioning is determined by Table 1. For methods to determine the amounts of spacing reduction and addition by line adjustments, see § D. Opportunities for Inter-character Space Reduction during Line Adjustment and § E. Opportunities for Inter-character Space Expansion during Line Adjustment .",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [0, 0, 0, 0], 0, 0),
        statement: "Line break opportunities between two adjacent characters of given character classes explained in § 3.9.2 Grouping of Characters and Symbols depending on their Positioning shall be determined by Table 2.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [3, 0, 0, 0], 1, 0),
        statement: "As noted in § B. Spacing between Characters and § C.2 Notes , there are several conventions for line-start prohibition, line-end prohibition and unbreakable character rules. The following lists four levels of convention. Note that breaking a line after opening brackets (cl-01) and before closing brackets (cl-02), full stops (cl-06) or commas (cl-07) is prohibited at all levels. Likewise, those rules common to all levels are not listed below.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::D), [0, 0, 0, 0], 0, 0),
        statement: "The following tables indicate if an opportunity exists for inter-character spacing reduction during line adjustment between two adjacent characters of given character classes as explained in § 3.9.2 Grouping of Characters and Symbols depending on their Positioning . (For more detail on line adjustment, see § 3.8 Line Adjustment .) In the process of line adjustment by inter-character spacing reduction, the first place to look (the first stage of inter-character spacing reduction in priority order) is for Western word spaces (cl-26), each of which is reducible equally, to leave a minimum of a quarter em spacing (or a one fifth em spacing) with respect to the corresponding character size. The tables are for the second and subsequent stages of inter-character spacing reduction in priority order, assuming the first stage of the reduction for Western word spaces (cl-26) is already done. The default unadjusted spacing between two adjacent characters of given character classes shall be determined according to § B. Spacing between Characters .",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::D), [1, 0, 0, 0], 1, 0),
        statement: "Note that JIS X 4051 specifies to not leave any spacing after the closing brackets (cl-02) or commas (cl-07) at the line end. Therefore, Table 4 also indicates that there is no opportunity for spacing reduction after closing brackets (cl-02) and commas (cl-07) at the line end. Likewise, because middle dots (cl-05) at the line end are supposed to have no spacing, Table 4 indicates there is no opportunity for spacing reduction for middle dots (cl-05) at the line end. On the other hand, while JIS X 4051 specifies to pad with a half em spacing after full stops (cl-06) at the line end, which is not allowed to reduce this spacing for line adjustment, Table 3 and 5 allow the removal of the default half em spacing after closing brackets (cl-02), full stops (cl-06) and commas (cl-07) at the line end for line adjustment. Table 3 further allows the removal of the default quarter em spacing padding before and after middle dots (cl-05) at the line end for line adjustment, while Table 5 does not.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [0, 0, 0, 0], 0, 0),
        statement: "The following table indicates if an opportunity exists for inter-character spacing expansion during line adjustment between two adjacent characters of given character classes as explained in § 3.9.2 Grouping of Characters and Symbols depending on their Positioning . (For more detail on line adjustment, see § 3.8 Line Adjustment .) In the process of line adjustment by inter-character spacing expansion, the first place to look (the first stage of inter-character spacing expansion in priority order) is for Western word spaces (cl-26), each of which is expandable equally, to take up and maximum of a half em space with respect to the corresponding character size. The tables are for the second and subsequent stages of inter-character spacing expansion in priority order, assuming the first stage of the expansion for Western word spaces (cl-26) is already done. The default unadjusted space between two adjacent characters of given character classes shall be determined according to § B. Spacing between Characters .",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::F), [0, 0, 0, 0], 0, 0),
        statement: "Positioning of ruby characters is explained in § 3.3 Ruby and Emphasis Dots , including that of jukugo-ruby, however it is limited to the basic principles. This appendix provides supplementary notes on jukugo-ruby distribution in terms of the structure of a kanji compound word (jukugo) and the type of script of the characters adjacent to the kanji compound word. All explanations hereafter in this appendix assume we are going to compose ruby characters with 'katatsuki' distribution (top-alignment in vertical writing mode).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::F), [1, 0, 0, 0], 1, 0),
        statement: "The following are principles of jukugo-ruby distribution, taking account of the structure of a kanji compound word and the type of script of the adjacent characters surrounding the compound word.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::F), [2, 0, 0, 0], 1, 0),
        statement: "In letterpress printing, ruby text was composed according to \"principles part one\" in the previous section, but on a case by case basis. Therefore, ruby texts were often composed differently for the same kanji compound word in the same situation. In some cases they differed according to the person in charge of the composition. In this section, one consistent method of ruby composition is presented as \"principles part two\", which is established with reference to those adopted in books and other publications.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::F), [3, 0, 0, 0], 1, 0),
        statement: "Principles for a method of jukugo-ruby distribution which allows inter-character spacing to expand, are as follows.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::F), [4, 0, 0, 0], 1, 0),
        statement: "The following are examples of jukugo-ruby distribution in accordance with the principles mentioned in the previous section.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 1),
        statement: "When opening brackets (cl-01) are followed by a simple-ruby character complex (cl-22) or jukugo-ruby character complex (cl-23), the preferred approach is to allow the ruby text to be extended up to the size of the ruby character over the opening brackets (cl-01). One alternative approach is to not allow ruby text to be extended over opening brackets, and another is to allow it to be extended up to half the size of the ruby character.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 2),
        statement: "The preferred spacing between closing brackets (cl-02) and the line end is a half em. The alternative is to set solid (JIS X 4051 adopts solid setting method, see § 3.1.9 Positioning of Closing Brackets, Full Stops, Commas and Middle Dots at Line End ).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 3),
        statement: "Character spacing between two consecutive middle dots (cl-05) shall be the sum of a quarter em of the preceding middle dots and a quarter em of the trailing middle dots.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 4),
        statement: "The preferred spacing between middle dots (cl-05) and the line end is a quarter em. The alternative is to set solid (JIS X 4051 adopts solid setting method, see § 3.1.9 Positioning of Closing Brackets, Full Stops, Commas and Middle Dots at Line End ).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 5),
        statement: "In this document, character spacing between a full stop (cl-06) or comma (cl-07) and a following middle dot (cl-05) is the sum of the half em spacing of the full stop or comma and the quarter em spacing of the middle dot. On the other hand, JIS X 4051 classifies commas (cl-07) as a subset of closing brackets (cl-02), and, therefore, where a comma (cl-07) is followed by a middle dot (cl-05) in JIS X 4051 the character spacing between them is just the quarter em spacing of the following middle dot (cl-05).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 6),
        statement: "The preferred spacing between full stops (cl-06) or commas (cl-07) and the line end is a half em. The alternative is to set solid (JIS X 4051 specifies that the spacing after full stop (cl-06) is a half em and the spacing after comma (cl-07) is solid, see § 3.1.9 Positioning of Closing Brackets, Full Stops, Commas and Middle Dots at Line End ).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 7),
        statement: "When a simple-ruby character complex (cl-22) or jukugo-ruby character complex (cl-23) is adjacent to katakana (cl-16), the preferred approach is to allow the ruby text to be extended up to the size of the ruby character over the katakana. However, if it is required to conform to JIS X 4051, ruby text shall not be extended over the katakana because katakana characters belong to the ideographic character class in JIS X 4051. There are alternative methods, one of which is to allow ruby text to be extended up to the size of the ruby character over any character including ideographic (cl-19) as well as hiragana (cl-15) and katakana (cl-16) characters, and another is NOT to allow ruby text to be extended over any character from hiragana (cl-15), katakana (cl-16) and ideographic characters (cl-19).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 8),
        statement: "Ruby text can be extended up to the size of the ruby character over the full-width ideographic space (cl-14). The preferred approach is to apply the same for the full-width line head indent at the beginning of a paragraph. The alternative approach is not to allow ruby text to be extended over the line head indent.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 9),
        statement: "When two adjacent characters belong to the same ornamented character complex (cl-21) run, set them according to the method explained in § 3.7.1 Superscripts and Superscripts . When two adjacent characters belong to two distinct ornamented character complex runs, set them solid.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 10),
        statement: "When two adjacent characters belong to the same simple-ruby character complex (cl-22) run, set them according to the method explained in § 3.3.5 Positioning of Mono-ruby with Respect to Base Characters or § 3.3.6 Positioning of Group-ruby with Respect to Base Characters . When two adjacent characters belong to two distinct simple-ruby character complex runs, set them solid.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 11),
        statement: "When two adjacent characters belong to the same jukugo-ruby character complex (cl-23) run, set them according to the method explained in § 3.3.7 Positioning of Jukugo-ruby with Respect to Base Characters . When two adjacent characters belong to two distinct simple-ruby character complex runs, set them solid.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 12),
        statement: "Character spacing between a preceding unit symbol (cl-25) and a trailing middle dot (cl-05) shall be a quarter em of the trailing character. Note that KATAKANA MIDDLE DOT \"・\" can be used either as a unit symbol (cl-25) or as a middle dot. When it is used as a unit symbol (cl-25), both preceding and trailing spacing of KATAKANA MIDDLE DOT \"・\" shall be zero.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 13),
        statement: "There shall be no visible space occupied by Western word space (cl-26) at the line head and that of warichu (inline cutting note), the line end and that of warichu (inline cutting note). If the condition is changed for the same text, restore the default visible space for Western word space (cl-26).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 14),
        statement: "In principle iteration marks (cl-09) should be placed neither at the line head nor at the head of an inline cutting note. When it happens with IDEOGRAPHIC ITERATION MARK \"々\", there are three ways to deal with this situation. Follow the principle by applying some sort of line adjustment. In this case, IDEOGRAPHIC ITERATION MARK \"々\" remains in iteration marks (cl-09). Allow IDEOGRAPHIC ITERATION MARK \"々\" to be placed either at the line head or at the head of an inline cutting note. In this case, the character shall be treated as part of the ideographic characters (cl-19) class. Replace IDEOGRAPHIC ITERATION MARK \"々\" with the corresponding character. line end: 国 line head: 々 <is replaced with> line end: 国 line head: 国 line end: 人 line head: 々 <is replaced with> line end: 人 line head: 人",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 15),
        statement: "In principle, a prolonged sound mark (cl-10) should be placed neither at the head of a line nor that of an inline cutting note. If it were allowed, the character shall be treated as part of the katakana (cl-16) class.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 16),
        statement: "In principle, small kana (cl-11) should be placed neither at the head of a line nor that of an inline cutting note in principle. If it were to be allowed, HIRAGANA LETTER SMALL * shall be treated as part of the hiragana (cl-15) class, and KATAKANA LETTER SMALL * as part of the katakana (cl-16) class.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::B), [2, 0, 0, 0], 1, 17),
        statement: "The preferred character spacing between the line head and opening opening brackets (cl-01) is zero. An alternative way is not to remove a conditional half em spacing accompanying the characters (see § 3.1.5 Positioning of Opening Brackets at Line Head including methods of positioning of opening brackets at the beginning of paragraphs).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 1),
        statement: "If IDEOGRAPHIC ITERATION MARK \"々\" is allowed to appear at the line head or that of inline cutting note, the character shall be treated as a member of the ideographic character (cl-19) class. (For how it behaves in combination with other character classes, see the cells for ideographic characters (cl-19).)",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 2),
        statement: "If a prolonged sound mark (cl-10) is allowed to appear at the head of a line or that of inline cutting note, the character shall be treated as a member of the katakana (cl-16) class. (For how it behaves in combination with other character classes, see the cells for katakana (cl-16).)",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 3),
        statement: "If small kana (cl-11) are allowed to appear at the head of a line or that of inline cutting note, the character shall be treated as a member of the hiragana (cl-15) or katakana (cl-16) classes accordingly, depending on the script type of the character. (For how it behaves in combination with other character classes, see the cells for hiragana (cl-15) or katakana (cl-16).)",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 4),
        statement: "For the default one em spacing after dividing punctuation marks (cl-04) at the end of a sentence, full-width ideographic space (cl-14) can be used. See § 3.1.6 Positioning of Dividing Punctuation Marks (Question Mark and Exclamation Mark) and Hyphens for more detail on how to deal with this case.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 5),
        statement: "There is no line break opportunity between following couple of consecutive inseparable characters (cl-08) as follows: EM DASH \"―\", EM DASH \"―\" HORIZONTAL ELLIPSIS \"…\", HORIZONTAL ELLIPSIS \"…\" TWO DOT LEADER \"‥\", TWO DOT LEADER \"‥\" VERTICAL KANA REPEAT MARK UPPER HALF \"〳\", VERTICAL KANA REPEAT MARK LOWER HALF \"〵\" VERTICAL KANA REPEAT WITH VOICED SOUND MARK UPPER HALF \"〴\", VERTICAL KANA REPEAT MARK LOWER HALF \"〵\" When the combination of preceding inseparable characters (cl-08) and the trailing inseparable characters (cl-08) is different each other, the two characters are separable. For example, when two EM DASH \"―\" appears consecutively, these two characters are inseparable, and consecutive EM DASH \"―\" and HORIZONTAL ELLIPSIS \"…\" are separable.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 6),
        statement: "There is no line break opportunity between two consecutive characters belonging to the same ornamented character complex (cl-21). If two consecutive characters belong to different ornamented character complexes (cl-21), a line break opportunity exists between them.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 7),
        statement: "There is no line break opportunity between two consecutive characters belonging to the same simple-ruby character complex (cl-22). If two consecutive characters belong to different simple-ruby character complexes (cl-22), a line break opportunity exists between them.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 8),
        statement: "A line break opportunity exists between two consecutive base characters belonging to different jukugo-ruby character complexes (cl-23). There is also a line break opportunity between two consecutive base characters belonging to the same jukugo-ruby character complex (cl-23) and between two runs of ruby text accompanying the corresponding base characters. However, a base character and the accompanying ruby text shall be indivisible, hence there is no line break opportunity between any two consecutive ruby characters in a run of ruby text accompanying a base character.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 9),
        statement: "There is no line break opportunity between preceding grouped numerals (cl-24) and trailing postfixed abbreviations (cl-13). The alternative approach is to allow a line to break before trailing PERCENT SIGN \"%\", in which case PERCENT SIGN \"%\" shall be treated as a member of the ideographic character (cl-19) class.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 10),
        statement: "There are two approaches: one is to allow a line to break between preceding grouped numerals (cl-24) and trailing Western characters (cl-27), and the other is not to.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 11),
        statement: "A line break opportunity generally exists between preceding Western characters (cl-27) and trailing postfixed abbreviations (cl-13), unless the preceding Western character (cl-27) is used as a symbol of a quantity or a European numeral, in which case a line break is not allowed between them.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 12),
        statement: "There is no line break opportunity between two consecutive Western characters (cl-27). In order to break a line in the middle of a Western word, it needs to be divided into two syllables first. Then a line can be broken between the two by adding HYPHEN \"-\" at the line end.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::C), [2, 0, 0, 0], 1, 13),
        statement: "There is no line break opportunity between two consecutive characters belonging to the same set of characters in tate-chu-yoko (cl-30). If two consecutive characters belong to different sets of characters in tate-chu-yoko (cl-30), there a line break opportunity exists between them.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::D), [2, 0, 0, 0], 1, 1),
        statement: "The default unadjusted spacing when a middle dot (cl-05) is followed by a middle dot (cl-05), is the sum of the conditional quarter em space accompanying the preceding middle dot (cl-05) and the conditional quarter em space accompanying the trailing middle dot (cl-05). Tables 3 and 4 allow these two instances of quarter em space to be reduced, to leave no space as a minimum. The priority order in space reduction is the fourth in Table 3, and it is the second priority in Table 4.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::D), [2, 0, 0, 0], 1, 2),
        statement: "The default unadjusted space when a full stop (cl-06) is followed by a middle dot (cl-05), is the sum of the conditional half em space accompanying the preceding full stop (cl-06) and the conditional quarter em space accompanying the trailing middle dot (cl-05). Tables 3 and 4 allow the quarter em space accompanying the trailing middle dot (cl-05) to be reduced, to leave no space as a minimum. The priority order in space reduction is the fourth in Table 3, and it is the second priority in Table 4.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::D), [2, 0, 0, 0], 1, 3),
        statement: "The default unadjusted space when a comma (cl-07) is followed by a middle dot (cl-05), is the sum of the conditional half em space accompanying the preceding comma (cl-07) and the conditional quarter em space accompanying the trailing middle dot (cl-05) (in Table 4, the conditional half space accompanying preceding comma (cl-07) and the conditional quarter space accompanying trailing middle dot (cl-05) can be reduced to solid setting). Table 5 allows the conditional half em space accompanying preceding comma (cl-07) to be reduced to a quarter space as a minimum. The priority order in space reduction for the conditional space accompanying middle dots (cl-05) is the fourth in Table 3 and the second in Table 4. The priority order in space reduction for the conditional space accompanying comma (cl-07) is the fifth in Table 3 and the third in Table 5.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::D), [2, 0, 0, 0], 1, 4),
        statement: "There is no opportunity for space reduction for a Western word space (cl-26) at the line head and at the line end since there is supposed to be no visible space. The same applies to the Western word space (cl-26) at the line head or the line end of warichu (inline cutting note). If the condition is changed for the same text, restore the default visible space for Western word space (cl-26).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::D), [2, 0, 0, 0], 1, 5),
        statement: "Table 3, and only Table 3, allows the preceding and trailing conditional quarter em space accompanying middle dots (cl-05) to be reduced to leave no space. The priority order is the third.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 1),
        statement: "If the IDEOGRAPHIC ITERATION MARK \"々\" is allowed to appear at the head of a line or that of inline cutting note, the character shall be treated as a member of the ideographic character (cl-19) class. (For how it behaves in combination with other character classes, see the cells for ideographic characters (cl-19).)",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 2),
        statement: "If a prolonged sound mark (cl-10) is allowed to appear at the line head or that of inline cutting note, the character shall be treated as a member of the katakana (cl-16) class. (For how it behaves in combination with other character classes, see the cells for katakana (cl-16).)",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 3),
        statement: "If small kana (cl-11) are allowed to appear at the head of a line or that of inline cutting note, the character shall be treated as a member of the hiragana (cl-15) or katakana (cl-16) class accordingly, depending on the script type of the character. (For how it behaves in combination with other character classes, see the cells for hiragana (cl-15) or katakana (cl-16).)",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 4),
        statement: "A third order opportunity exists for inter-character spacing expansion, to take up to a maximum of a quarter em space, with respect to the corresponding character size, between two consecutive inseparable characters (cl-08) which are of different kinds.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 5),
        statement: "A third order opportunity exists for inter-character spacing expansion, to take up to a maximum of a quarter em space, with respect to the corresponding character size, between the two consecutive characters which belong to different ornamented character complexes (cl-21)",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 6),
        statement: "A third order opportunity exists for inter-character spacing expansion, to take up to a maximum of a quarter em space, with respect to the corresponding character size, if the two consecutive characters belong to different simple-ruby character complexes (cl-22). If not, inter-character spacing expansion is not allowed.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 7),
        statement: "A third order opportunity exists for inter-character spacing expansion, to take up to a maximum of a quarter em space, with respect to the corresponding character size, if the two consecutive base characters belonging to different jukugo-ruby character complexes (cl-23). If not, inter-character spacing expansion is not allowed.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 8),
        statement: "There is no opportunity for inter-character spacing expansion between a preceding grouped numeral (cl-24) and a trailing postfixed abbreviation (cl-13), unless the alternative approach is chosen which allows a line to break between a preceding grouped numeral (cl-24) and the trailing PERCENT SIGN \"%\", where PERCENT SIGN \"%\" shall be treated as a member of the ideographic character (cl-19) class, in front of which inter-character spacing is expandable.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 9),
        statement: "There is an alternative way to give a third order opportunity for inter-character spacing expansion, to take up to a maximum of a quarter em space, with respect to the corresponding character size, between a preceding grouped numeral (cl-24) and a trailing Western character (cl-27).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 10),
        statement: "A third order opportunity exists for inter-character spacing expansion between a preceding Western character (cl-27) and a trailing postfixed abbreviation (cl-13), unless the preceding Western character (cl-27) is used as a symbol of a quantity or a European numeral, in which case no inter-character spacing expansion is allowed between them.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 11),
        statement: "There is an alternative way to give a fourth order opportunity for inter-character spacing expansion with respect to the corresponding character size, between two consecutive Western characters (cl-27).",
        direction_conditional: false,
        standing: Standing::Normative,
    },
    Rule {
        address: Address::assembled(Some(Appendix::E), [2, 0, 0, 0], 1, 12),
        statement: "A third order opportunity exists for the inter-character spacing expansion, to take up to a maximum of a quarter em space, with respect to the corresponding character size, if two consecutive characters belong to different runs of characters in tate-chu-yoko (cl-30). If not, inter-character spacing expansion is not allowed.",
        direction_conditional: false,
        standing: Standing::Normative,
    },
];

impl RuleId {
    /// The statement JLReq makes at §3.1.1.
    ///
    /// JLReq: §3.1.1
    pub const DIFFERENCES_IN_VERTICAL_AND_HORIZONTAL_COMPOSITION_IN_USE_OF_PUNCTUATION_MARKS: Self =
        Self(0);

    /// The statement JLReq makes at §3.1.2.
    ///
    /// JLReq: §3.1.2
    pub const POSITIONING_OF_PUNCTUATION_MARKS_COMMAS_PERIODS_AND_BRACKETS: Self = Self(1);

    /// The statement JLReq makes at §3.1.3.
    ///
    /// JLReq: §3.1.3
    pub const EXCEPTIONAL_POSITIONING_OF_IDEOGRAPHIC_COMMA_AND_KATAKANA_MIDDLE_DOT: Self = Self(2);

    /// The statement JLReq makes at §3.1.4.
    ///
    /// JLReq: §3.1.4
    pub const POSITIONING_OF_CONSECUTIVE_OPENING_BRACKETS_CLOSING_BRACKETS_COMMAS_FULL_STOPS_AND_MIDDLE_DOTS: Self =
        Self(3);

    /// The statement JLReq makes at §3.1.5.
    ///
    /// JLReq: §3.1.5
    pub const POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD: Self = Self(4);

    /// The statement JLReq makes at §3.1.6.
    ///
    /// JLReq: §3.1.6
    pub const POSITIONING_OF_DIVIDING_PUNCTUATION_MARKS_QUESTION_MARK_AND_EXCLAMATION_MARK_AND_HYPHENS: Self =
        Self(5);

    /// The statement JLReq makes at §3.1.7.
    ///
    /// JLReq: §3.1.7
    pub const CHARACTERS_NOT_STARTING_A_LINE: Self = Self(6);

    /// The statement JLReq makes at §3.1.8.
    ///
    /// JLReq: §3.1.8
    pub const CHARACTERS_NOT_ENDING_A_LINE: Self = Self(7);

    /// The statement JLReq makes at §3.1.9.
    ///
    /// JLReq: §3.1.9
    pub const POSITIONING_OF_CLOSING_BRACKETS_FULL_STOPS_COMMAS_AND_MIDDLE_DOTS_AT_LINE_END: Self =
        Self(8);

    /// The statement JLReq makes at §3.1.10.
    ///
    /// JLReq: §3.1.10
    pub const UNBREAKABLE_CHARACTER_SEQUENCES: Self = Self(9);

    /// The statement JLReq makes at §3.1.11.
    ///
    /// JLReq: §3.1.11
    pub const CHARACTER_SEQUENCES_WHICH_DO_NOT_ALLOW_INCREASE_OF_SPACING_AS_PART_OF_LINE_ADJUSTMENT_PROCESSING: Self =
        Self(10);

    /// The statement JLReq makes at §3.1.12.
    ///
    /// JLReq: §3.1.12
    pub const EXAMPLES_OF_LINE_ADJUSTMENT: Self = Self(11);

    /// The statement JLReq makes at §3.2.1.
    ///
    /// JLReq: §3.2.1
    pub const COMPOSITION_OF_JAPANESE_AND_WESTERN_MIXED_TEXTS: Self = Self(12);

    /// The statement JLReq makes at §3.2.2.
    ///
    /// JLReq: §3.2.2
    pub const MIXED_TEXT_COMPOSITION_IN_HORIZONTAL_WRITING_MODE: Self = Self(13);

    /// The statement JLReq makes at §3.2.3.
    ///
    /// JLReq: §3.2.3
    pub const MIXED_TEXT_COMPOSITION_IN_VERTICAL_WRITING_MODE: Self = Self(14);

    /// The statement JLReq makes at §3.2.4.
    ///
    /// JLReq: §3.2.4
    pub const METHOD_FOR_SETTING_FULL_WIDTH_LATIN_LETTERS_AND_EUROPEAN_NUMERALS: Self = Self(15);

    /// The statement JLReq makes at §3.2.5.
    ///
    /// JLReq: §3.2.5
    pub const HANDLING_OF_TATE_CHU_YOKO_HORIZONTAL_IN_VERTICAL_SETTINGS: Self = Self(16);

    /// The statement JLReq makes at §3.2.6.
    ///
    /// JLReq: §3.2.6
    pub const HANDLING_OF_WESTERN_TEXT_IN_JAPANESE_TEXT_USING_PROPORTIONAL_WESTERN_FONTS: Self =
        Self(17);

    /// The statement JLReq makes at §3.3.1.
    ///
    /// JLReq: §3.3.1
    pub const USAGE_OF_RUBY: Self = Self(18);

    /// The statement JLReq makes at §3.3.2.
    ///
    /// JLReq: §3.3.2
    pub const CHOICE_OF_BASE_CHARACTERS_TO_BE_ANNOTATED_BY_RUBY: Self = Self(19);

    /// The statement JLReq makes at §3.3.3.
    ///
    /// JLReq: §3.3.3
    pub const CHOICE_OF_SIZE_FOR_RUBY_CHARACTERS: Self = Self(20);

    /// The statement JLReq makes at §3.3.4.
    ///
    /// JLReq: §3.3.4
    pub const CHOICE_OF_SIDES_FOR_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS: Self = Self(21);

    /// The statement JLReq makes at §3.3.5.
    ///
    /// JLReq: §3.3.5
    pub const POSITIONING_OF_MONO_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS: Self = Self(22);

    /// The statement JLReq makes at §3.3.6.
    ///
    /// JLReq: §3.3.6
    pub const POSITIONING_OF_GROUP_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS: Self = Self(23);

    /// The statement JLReq makes at §3.3.7.
    ///
    /// JLReq: §3.3.7
    pub const POSITIONING_OF_JUKUGO_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS: Self = Self(24);

    /// The statement JLReq makes at §3.3.8.
    ///
    /// JLReq: §3.3.8
    pub const ADJUSTMENTS_OF_RUBY_WITH_LENGTH_LONGER_THAN_THAT_OF_THE_BASE_CHARACTERS: Self =
        Self(25);

    /// The statement JLReq makes at §3.3.9.
    ///
    /// JLReq: §3.3.9
    pub const COMPOSITION_OF_EMPHASIS_DOTS: Self = Self(26);

    /// The statement JLReq makes at §3.4.1.
    ///
    /// JLReq: §3.4.1
    pub const WHERE_THE_INLINE_CUTTING_NOTE_WARICHU_IS_USED: Self = Self(27);

    /// The statement JLReq makes at §3.4.2.
    ///
    /// JLReq: §3.4.2
    pub const CHARACTER_SIZE_FOR_INLINE_CUTTING_NOTES_AND_LINE_GAPS: Self = Self(28);

    /// The statement JLReq makes at §3.4.3.
    ///
    /// JLReq: §3.4.3
    pub const HANDLING_AN_INLINE_CUTTING_NOTE_WHEN_IT_STRADDLES_TWO_KIHON_HANMEN_LINES: Self =
        Self(29);

    /// The statement JLReq makes at §3.5.1.
    ///
    /// JLReq: §3.5.1
    pub const LINE_HEAD_INDENT_AT_THE_BEGINNING_OF_PARAGRAPHS: Self = Self(30);

    /// The statement JLReq makes at §3.5.2.
    ///
    /// JLReq: §3.5.2
    pub const LINE_HEAD_INDENT_AND_LINE_END_INDENT: Self = Self(31);

    /// The statement JLReq makes at §3.5.3.
    ///
    /// JLReq: §3.5.3
    pub const SINGLE_LINE_ALIGNMENT_PROCESSING: Self = Self(32);

    /// The statement JLReq makes at §3.5.4.
    ///
    /// JLReq: §3.5.4
    pub const WIDOW_ADJUSTMENT_OF_PARAGRAPHS: Self = Self(33);

    /// The statement JLReq makes at §3.6.1.
    ///
    /// JLReq: §3.6.1
    pub const USAGE_OF_TAB_SETTING: Self = Self(34);

    /// The statement JLReq makes at §3.6.2.
    ///
    /// JLReq: §3.6.2
    pub const TYPES_OF_TAB_SETTINGS: Self = Self(35);

    /// The statement JLReq makes at §3.6.3.
    ///
    /// JLReq: §3.6.3
    pub const THE_METHOD_OF_SETTING_THE_TARGET_TEXT: Self = Self(36);

    /// The statement JLReq makes at §3.7.1.
    ///
    /// JLReq: §3.7.1
    pub const SUPERSCRIPTS_AND_SUPERSCRIPTS: Self = Self(37);

    /// The statement JLReq makes at §3.7.2.
    ///
    /// JLReq: §3.7.2
    pub const FURIWAKE_PROCESSING: Self = Self(38);

    /// The statement JLReq makes at §3.7.3.
    ///
    /// JLReq: §3.7.3
    pub const JIDORI_PROCESSING: Self = Self(39);

    /// The statement JLReq makes at §3.7.4.
    ///
    /// JLReq: §3.7.4
    pub const PROCESSING_OF_MATH_SYMBOLS_AND_MATH_OPERATORS: Self = Self(40);

    /// The statement JLReq makes at §3.8.1.
    ///
    /// JLReq: §3.8.1
    pub const NECESSITY_FOR_LINE_ADJUSTMENT: Self = Self(41);

    /// The statement JLReq makes at §3.8.2.
    ///
    /// JLReq: §3.8.2
    pub const REDUCTION_AND_ADDITION_OF_INTER_CHARACTER_SPACING: Self = Self(42);

    /// The statement JLReq makes at §3.8.3.
    ///
    /// JLReq: §3.8.3
    pub const PROCEDURES_FOR_INTER_CHARACTER_SPACING_REDUCTION: Self = Self(43);

    /// The statement JLReq makes at §3.8.4.
    ///
    /// JLReq: §3.8.4
    pub const PROCEDURES_FOR_INTER_CHARACTER_SPACE_EXPANSION: Self = Self(44);

    /// The statement JLReq makes at §3.9.1.
    ///
    /// JLReq: §3.9.1
    pub const DIFFERENCES_IN_POSITIONING_OF_CHARACTERS_AND_SYMBOLS: Self = Self(45);

    /// The statement JLReq makes at §3.9.2.
    ///
    /// JLReq: §3.9.2
    pub const GROUPING_OF_CHARACTERS_AND_SYMBOLS_DEPENDING_ON_THEIR_POSITIONING: Self = Self(46);

    /// The statement JLReq makes at §3.9.3.
    ///
    /// JLReq: §3.9.3
    pub const POSITIONING_METHODS_FOR_EACH_CHARACTER_CLASS: Self = Self(47);

    /// The statement JLReq makes at §B.
    ///
    /// JLReq: §B
    pub const SPACING_BETWEEN_CHARACTERS: Self = Self(48);

    /// The statement JLReq makes at §C.
    ///
    /// JLReq: §C
    pub const POSSIBILITIES_FOR_LINE_BREAKING_BETWEEN_CHARACTERS: Self = Self(49);

    /// The statement JLReq makes at §C.3.
    ///
    /// JLReq: §C.3
    pub const ADDENDUM: Self = Self(50);

    /// The statement JLReq makes at §D.
    ///
    /// JLReq: §D
    pub const OPPORTUNITIES_FOR_INTER_CHARACTER_SPACE_REDUCTION_DURING_LINE_ADJUSTMENT: Self =
        Self(51);

    /// The statement JLReq makes at §D.1.
    ///
    /// JLReq: §D.1
    pub const LEGEND_OF_TABLES_3_4_AND_5: Self = Self(52);

    /// The statement JLReq makes at §E.
    ///
    /// JLReq: §E
    pub const OPPORTUNITIES_FOR_INTER_CHARACTER_SPACE_EXPANSION_DURING_LINE_ADJUSTMENT: Self =
        Self(53);

    /// The statement JLReq makes at §F.
    ///
    /// JLReq: §F
    pub const POSITIONING_OF_JUKUGO_RUBY: Self = Self(54);

    /// The statement JLReq makes at §F.1.
    ///
    /// JLReq: §F.1
    pub const PRINCIPLES_OF_JUKUGO_RUBY_DISTRIBUTION_PART_1: Self = Self(55);

    /// The statement JLReq makes at §F.2.
    ///
    /// JLReq: §F.2
    pub const PRINCIPLES_OF_JUKUGO_RUBY_DISTRIBUTION_PART_2: Self = Self(56);

    /// The statement JLReq makes at §F.3.
    ///
    /// JLReq: §F.3
    pub const PRINCIPLES_OF_JUKUGO_RUBY_DISTRIBUTION_WITH_INTER_CHARACTER_SPACE_EXPANSION: Self =
        Self(57);

    /// The statement JLReq makes at §F.4.
    ///
    /// JLReq: §F.4
    pub const EXAMPLES_OF_JUKUGO_RUBY_DISTRIBUTION_WITH_INTER_CHARACTER_SPACE_EXPANSION: Self =
        Self(58);

    /// The statement JLReq makes at §B.2#1.
    ///
    /// JLReq: §B.2#1
    pub const B_2_NOTE_1: Self = Self(59);

    /// The statement JLReq makes at §B.2#2.
    ///
    /// JLReq: §B.2#2
    pub const B_2_NOTE_2: Self = Self(60);

    /// The statement JLReq makes at §B.2#3.
    ///
    /// JLReq: §B.2#3
    pub const B_2_NOTE_3: Self = Self(61);

    /// The statement JLReq makes at §B.2#4.
    ///
    /// JLReq: §B.2#4
    pub const B_2_NOTE_4: Self = Self(62);

    /// The statement JLReq makes at §B.2#5.
    ///
    /// JLReq: §B.2#5
    pub const B_2_NOTE_5: Self = Self(63);

    /// The statement JLReq makes at §B.2#6.
    ///
    /// JLReq: §B.2#6
    pub const B_2_NOTE_6: Self = Self(64);

    /// The statement JLReq makes at §B.2#7.
    ///
    /// JLReq: §B.2#7
    pub const B_2_NOTE_7: Self = Self(65);

    /// The statement JLReq makes at §B.2#8.
    ///
    /// JLReq: §B.2#8
    pub const B_2_NOTE_8: Self = Self(66);

    /// The statement JLReq makes at §B.2#9.
    ///
    /// JLReq: §B.2#9
    pub const B_2_NOTE_9: Self = Self(67);

    /// The statement JLReq makes at §B.2#10.
    ///
    /// JLReq: §B.2#10
    pub const B_2_NOTE_10: Self = Self(68);

    /// The statement JLReq makes at §B.2#11.
    ///
    /// JLReq: §B.2#11
    pub const B_2_NOTE_11: Self = Self(69);

    /// The statement JLReq makes at §B.2#12.
    ///
    /// JLReq: §B.2#12
    pub const B_2_NOTE_12: Self = Self(70);

    /// The statement JLReq makes at §B.2#13.
    ///
    /// JLReq: §B.2#13
    pub const B_2_NOTE_13: Self = Self(71);

    /// The statement JLReq makes at §B.2#14.
    ///
    /// JLReq: §B.2#14
    pub const B_2_NOTE_14: Self = Self(72);

    /// The statement JLReq makes at §B.2#15.
    ///
    /// JLReq: §B.2#15
    pub const B_2_NOTE_15: Self = Self(73);

    /// The statement JLReq makes at §B.2#16.
    ///
    /// JLReq: §B.2#16
    pub const B_2_NOTE_16: Self = Self(74);

    /// The statement JLReq makes at §B.2#17.
    ///
    /// JLReq: §B.2#17
    pub const B_2_NOTE_17: Self = Self(75);

    /// The statement JLReq makes at §C.2#1.
    ///
    /// JLReq: §C.2#1
    pub const C_2_NOTE_1: Self = Self(76);

    /// The statement JLReq makes at §C.2#2.
    ///
    /// JLReq: §C.2#2
    pub const C_2_NOTE_2: Self = Self(77);

    /// The statement JLReq makes at §C.2#3.
    ///
    /// JLReq: §C.2#3
    pub const C_2_NOTE_3: Self = Self(78);

    /// The statement JLReq makes at §C.2#4.
    ///
    /// JLReq: §C.2#4
    pub const C_2_NOTE_4: Self = Self(79);

    /// The statement JLReq makes at §C.2#5.
    ///
    /// JLReq: §C.2#5
    pub const C_2_NOTE_5: Self = Self(80);

    /// The statement JLReq makes at §C.2#6.
    ///
    /// JLReq: §C.2#6
    pub const C_2_NOTE_6: Self = Self(81);

    /// The statement JLReq makes at §C.2#7.
    ///
    /// JLReq: §C.2#7
    pub const C_2_NOTE_7: Self = Self(82);

    /// The statement JLReq makes at §C.2#8.
    ///
    /// JLReq: §C.2#8
    pub const C_2_NOTE_8: Self = Self(83);

    /// The statement JLReq makes at §C.2#9.
    ///
    /// JLReq: §C.2#9
    pub const C_2_NOTE_9: Self = Self(84);

    /// The statement JLReq makes at §C.2#10.
    ///
    /// JLReq: §C.2#10
    pub const C_2_NOTE_10: Self = Self(85);

    /// The statement JLReq makes at §C.2#11.
    ///
    /// JLReq: §C.2#11
    pub const C_2_NOTE_11: Self = Self(86);

    /// The statement JLReq makes at §C.2#12.
    ///
    /// JLReq: §C.2#12
    pub const C_2_NOTE_12: Self = Self(87);

    /// The statement JLReq makes at §C.2#13.
    ///
    /// JLReq: §C.2#13
    pub const C_2_NOTE_13: Self = Self(88);

    /// The statement JLReq makes at §D.2#1.
    ///
    /// JLReq: §D.2#1
    pub const D_2_NOTE_1: Self = Self(89);

    /// The statement JLReq makes at §D.2#2.
    ///
    /// JLReq: §D.2#2
    pub const D_2_NOTE_2: Self = Self(90);

    /// The statement JLReq makes at §D.2#3.
    ///
    /// JLReq: §D.2#3
    pub const D_2_NOTE_3: Self = Self(91);

    /// The statement JLReq makes at §D.2#4.
    ///
    /// JLReq: §D.2#4
    pub const D_2_NOTE_4: Self = Self(92);

    /// The statement JLReq makes at §D.2#5.
    ///
    /// JLReq: §D.2#5
    pub const D_2_NOTE_5: Self = Self(93);

    /// The statement JLReq makes at §E.2#1.
    ///
    /// JLReq: §E.2#1
    pub const E_2_NOTE_1: Self = Self(94);

    /// The statement JLReq makes at §E.2#2.
    ///
    /// JLReq: §E.2#2
    pub const E_2_NOTE_2: Self = Self(95);

    /// The statement JLReq makes at §E.2#3.
    ///
    /// JLReq: §E.2#3
    pub const E_2_NOTE_3: Self = Self(96);

    /// The statement JLReq makes at §E.2#4.
    ///
    /// JLReq: §E.2#4
    pub const E_2_NOTE_4: Self = Self(97);

    /// The statement JLReq makes at §E.2#5.
    ///
    /// JLReq: §E.2#5
    pub const E_2_NOTE_5: Self = Self(98);

    /// The statement JLReq makes at §E.2#6.
    ///
    /// JLReq: §E.2#6
    pub const E_2_NOTE_6: Self = Self(99);

    /// The statement JLReq makes at §E.2#7.
    ///
    /// JLReq: §E.2#7
    pub const E_2_NOTE_7: Self = Self(100);

    /// The statement JLReq makes at §E.2#8.
    ///
    /// JLReq: §E.2#8
    pub const E_2_NOTE_8: Self = Self(101);

    /// The statement JLReq makes at §E.2#9.
    ///
    /// JLReq: §E.2#9
    pub const E_2_NOTE_9: Self = Self(102);

    /// The statement JLReq makes at §E.2#10.
    ///
    /// JLReq: §E.2#10
    pub const E_2_NOTE_10: Self = Self(103);

    /// The statement JLReq makes at §E.2#11.
    ///
    /// JLReq: §E.2#11
    pub const E_2_NOTE_11: Self = Self(104);

    /// The statement JLReq makes at §E.2#12.
    ///
    /// JLReq: §E.2#12
    pub const E_2_NOTE_12: Self = Self(105);
}
