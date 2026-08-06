// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The thirty character classes, and a set of them.
//!
//! §3.9.2 closes the set at thirty and Appendix A enumerates the members of twenty-five of
//! them. The other five have a heading and no table, because their section text reads in
//! full "Any character may participate in …": membership there is a property of the
//! construct an occurrence sits inside rather than of the code point, which is the sharpest
//! instance of what `docs/adr/0008` decided.
//!
//! Every name and every enumerating section below is read out of the published document by
//! `cargo run -p xtask -- generate`; nothing in this module transcribes one.

use jlreq_spec::Address;

use crate::generated::class_name::CLASSES;

/// How many classes §3.9.2 closes the set at.
const COUNT: usize = 30;

/// A JLReq character class, cl-01 through cl-30.
///
/// The one exhaustive public enum in this workspace whose cardinality is ours to state:
/// §3.9.2 closes the set, and a caller legitimately matches all thirty. A catch-all arm
/// over character classes is exactly where a silently wrong default hides, so this type
/// is deliberately not `#[non_exhaustive]` (ADR-0012).
///
/// The line edges are *not* members. Appendix B gives them one row and one column, not a
/// symmetric axis value; see `jlreq_spacing::Before` and `jlreq_spacing::After`.
///
/// JLReq: §3.9.2, §A
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// cl-01, opening brackets (始め括弧類). JLReq: §A.1
    OpeningBracket,
    /// cl-02, closing brackets (終わり括弧類). JLReq: §A.2
    ClosingBracket,
    /// cl-03, hyphens (ハイフン類). JLReq: §A.3
    Hyphen,
    /// cl-04, dividing punctuation marks (区切り約物). JLReq: §A.4
    DividingPunctuation,
    /// cl-05, middle dots (中点類). JLReq: §A.5
    MiddleDot,
    /// cl-06, full stops (句点類). JLReq: §A.6
    FullStop,
    /// cl-07, commas (読点類). JLReq: §A.7
    Comma,
    /// cl-08, inseparable characters (分離禁止文字). JLReq: §A.8
    Inseparable,
    /// cl-09, iteration marks (繰返し記号). JLReq: §A.9
    IterationMark,
    /// cl-10, the prolonged sound mark (長音記号). JLReq: §A.10
    ProlongedSoundMark,
    /// cl-11, small kana (小書きの仮名). JLReq: §A.11
    SmallKana,
    /// cl-12, prefixed abbreviations (前置省略記号). JLReq: §A.12
    PrefixedAbbreviation,
    /// cl-13, postfixed abbreviations (後置省略記号). JLReq: §A.13
    PostfixedAbbreviation,
    /// cl-14, the full-width ideographic space (和字間隔). JLReq: §A.14
    IdeographicSpace,
    /// cl-15, hiragana (平仮名). JLReq: §A.15
    Hiragana,
    /// cl-16, katakana (片仮名). JLReq: §A.16
    Katakana,
    /// cl-17, math symbols (等号類). JLReq: §A.17
    MathSymbol,
    /// cl-18, math operators (演算記号). JLReq: §A.18
    MathOperator,
    /// cl-19, ideographic characters (漢字等). Contains 66 Cyrillic and 49 Greek
    /// letters; the name is JLReq's, and it is not a description. JLReq: §A.19
    Ideographic,
    /// cl-20, characters as reference marks (合印中の文字). JLReq: §A.20
    AsReferenceMark,
    /// cl-21, characters in an ornamented complex. JLReq: §A.21
    InOrnamentedComplex,
    /// cl-22, characters in a non-jukugo ruby complex. JLReq: §A.22
    InNonJukugoRubyComplex,
    /// cl-23, characters in a jukugo-ruby complex. JLReq: §A.23
    InJukugoRubyComplex,
    /// cl-24, grouped numerals (連数字中の文字). JLReq: §A.24
    InGroupedNumeral,
    /// cl-25, unit symbols (単位記号中の文字). JLReq: §A.25
    InUnitSymbol,
    /// cl-26, the Western word space (欧文間隔). JLReq: §A.26
    WesternWordSpace,
    /// cl-27, Western characters (欧文用文字). JLReq: §A.27
    Western,
    /// cl-28, warichu opening brackets (割注始め括弧類). JLReq: §A.28
    WarichuOpeningBracket,
    /// cl-29, warichu closing brackets (割注終わり括弧類). JLReq: §A.29
    WarichuClosingBracket,
    /// cl-30, characters in tate-chu-yoko (縦中横中の文字). JLReq: §A.30
    InTateChuYoko,
}

impl Class {
    /// Every class, in the order §3.9.2 lists them.
    ///
    /// JLReq: §3.9.2
    pub const ALL: [Self; COUNT] = [
        Self::OpeningBracket,
        Self::ClosingBracket,
        Self::Hyphen,
        Self::DividingPunctuation,
        Self::MiddleDot,
        Self::FullStop,
        Self::Comma,
        Self::Inseparable,
        Self::IterationMark,
        Self::ProlongedSoundMark,
        Self::SmallKana,
        Self::PrefixedAbbreviation,
        Self::PostfixedAbbreviation,
        Self::IdeographicSpace,
        Self::Hiragana,
        Self::Katakana,
        Self::MathSymbol,
        Self::MathOperator,
        Self::Ideographic,
        Self::AsReferenceMark,
        Self::InOrnamentedComplex,
        Self::InNonJukugoRubyComplex,
        Self::InJukugoRubyComplex,
        Self::InGroupedNumeral,
        Self::InUnitSymbol,
        Self::WesternWordSpace,
        Self::Western,
        Self::WarichuOpeningBracket,
        Self::WarichuClosingBracket,
        Self::InTateChuYoko,
    ];

    /// `1` through `30`. JLReq: §3.9.2
    #[must_use]
    pub const fn number(self) -> u8 {
        (self as u8).saturating_add(1)
    }

    /// The identifier JLReq uses in every rule sentence: `cl-01` … `cl-30`.
    ///
    /// JLReq: §3.9.2, §A
    #[must_use]
    pub const fn id(self) -> &'static str {
        CLASSES[self.ordinal()].id
    }

    /// JLReq's English name, generated from §3.9.2 rather than from an Appendix A
    /// heading — §A.19's heading names the class's own complement, 漢字等（漢字以外の例）,
    /// "ideographic characters (examples other than ideographs)", and §A.10 writes the
    /// prolonged sound mark in the singular where §3.9.2 writes it in the plural.
    ///
    /// JLReq: §3.9.2
    #[must_use]
    pub const fn name_en(self) -> &'static str {
        CLASSES[self.ordinal()].en
    }

    /// JLReq's Japanese name, from §3.9.2. JLReq: §3.9.2
    #[must_use]
    pub const fn name_ja(self) -> &'static str {
        CLASSES[self.ordinal()].ja
    }

    /// The Appendix A section enumerating this class, if it enumerates anything.
    /// Five classes enumerate nothing: their section text reads in full "Any character
    /// may participate in …". JLReq: §A.20–§A.23, §A.30
    #[must_use]
    pub const fn enumeration(self) -> Option<Address> {
        let section = CLASSES[self.ordinal()].enumeration;
        if section.is_empty() {
            None
        } else {
            Address::parse(section)
        }
    }

    /// The class JLReq numbers `n`, or `None` when `n` numbers no class.
    ///
    /// JLReq: §3.9.2
    #[must_use]
    pub const fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::OpeningBracket),
            2 => Some(Self::ClosingBracket),
            3 => Some(Self::Hyphen),
            4 => Some(Self::DividingPunctuation),
            5 => Some(Self::MiddleDot),
            6 => Some(Self::FullStop),
            7 => Some(Self::Comma),
            8 => Some(Self::Inseparable),
            9 => Some(Self::IterationMark),
            10 => Some(Self::ProlongedSoundMark),
            11 => Some(Self::SmallKana),
            12 => Some(Self::PrefixedAbbreviation),
            13 => Some(Self::PostfixedAbbreviation),
            14 => Some(Self::IdeographicSpace),
            15 => Some(Self::Hiragana),
            16 => Some(Self::Katakana),
            17 => Some(Self::MathSymbol),
            18 => Some(Self::MathOperator),
            19 => Some(Self::Ideographic),
            20 => Some(Self::AsReferenceMark),
            21 => Some(Self::InOrnamentedComplex),
            22 => Some(Self::InNonJukugoRubyComplex),
            23 => Some(Self::InJukugoRubyComplex),
            24 => Some(Self::InGroupedNumeral),
            25 => Some(Self::InUnitSymbol),
            26 => Some(Self::WesternWordSpace),
            27 => Some(Self::Western),
            28 => Some(Self::WarichuOpeningBracket),
            29 => Some(Self::WarichuClosingBracket),
            30 => Some(Self::InTateChuYoko),
            _ => None,
        }
    }

    /// This class's position in the generated table, which is its number less one.
    const fn ordinal(self) -> usize {
        self as usize
    }

    /// Whether membership of this class is a property of the construct an occurrence sits
    /// in rather than of the key.
    ///
    /// Nine classes: the five that enumerate nothing at all, and the four that enumerate
    /// the characters permitted *inside* a construct — a grouped numeral (連数字), a unit
    /// symbol, and the two warichu (割注) bracket classes. Appendix A listing a key under
    /// one of them says which characters may appear there, never that this occurrence is
    /// one, so nothing in a stream of items can rule the class in or out and the caller's
    /// construct axis is what decides it.
    ///
    /// Published rather than crate-visible, because it is what a caller needs to read a
    /// [`ClassSet`] this crate hands them: `Classified::Several` names `AxisSet::CONSTRUCT`
    /// when one of these survives, and `resolve` passes over them in its tie-break for the
    /// same reason. A caller who cannot ask the question cannot check either answer.
    ///
    /// JLReq: §A.20–§A.25, §A.28–§A.30
    #[must_use]
    pub const fn is_construct_membership(self) -> bool {
        matches!(
            self,
            Self::AsReferenceMark
                | Self::InOrnamentedComplex
                | Self::InNonJukugoRubyComplex
                | Self::InJukugoRubyComplex
                | Self::InGroupedNumeral
                | Self::InUnitSymbol
                | Self::WarichuOpeningBracket
                | Self::WarichuClosingBracket
                | Self::InTateChuYoko
        )
    }

    /// Whether §3.1.2 states this class's character advance as half-width.
    ///
    /// Commas (cl-07), full stops (cl-06), opening brackets (cl-01), closing brackets
    /// (cl-02) and middle dots (cl-05). There the frame (字幅) decides a geometry rather
    /// than a class, so an unstated frame has no answer to report and `Text::new` refuses
    /// the stream (ADR-0018).
    ///
    /// JLReq: §3.1.2
    pub(crate) const fn advance_is_stated_half_width(self) -> bool {
        matches!(
            self,
            Self::OpeningBracket
                | Self::ClosingBracket
                | Self::MiddleDot
                | Self::FullStop
                | Self::Comma
        )
    }
}

/// A set of classes, as a bitmask. Allocation-free and order-deterministic, which is why
/// it is not a `BTreeSet`.
///
/// Thirty classes fit in a `u32` with two bits to spare, so a set is one machine word and
/// iterating one answers in class-number order on every target — which is what a report and
/// a conformance case need, and what a hashed set could not give.
///
/// JLReq: §3.9.2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ClassSet(u32);

impl ClassSet {
    /// No class at all. JLReq: §3.9.2
    pub const EMPTY: Self = Self(0);

    /// Every class §3.9.2 defines. JLReq: §3.9.2
    pub const ALL: Self = {
        let mut set = Self::EMPTY;
        let mut index = 0;
        while index < COUNT {
            set = set.with(Class::ALL[index]);
            index = index.saturating_add(1);
        }
        set
    };

    /// The set holding one class. JLReq: §3.9.2
    #[must_use]
    pub const fn of(class: Class) -> Self {
        Self::EMPTY.with(class)
    }

    /// This set with `class` added. JLReq: §3.9.2
    #[must_use]
    pub const fn with(self, class: Class) -> Self {
        Self(self.0 | class.bit())
    }

    /// This set without `class`. JLReq: §3.9.2
    #[must_use]
    pub const fn without(self, class: Class) -> Self {
        Self(self.0 & !class.bit())
    }

    /// Whether `class` is in this set. JLReq: §3.9.2
    #[must_use]
    pub const fn contains(self, class: Class) -> bool {
        self.0 & class.bit() != 0
    }

    /// How many classes are in this set. JLReq: §3.9.2
    #[must_use]
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Whether no class is in this set. JLReq: §3.9.2
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The one class in this set, or `None` when it holds none or several.
    ///
    /// JLReq: §3.9.2
    #[must_use]
    pub const fn only(self) -> Option<Class> {
        if self.len() != 1 {
            return None;
        }
        // Read out of the class list rather than out of the bit index, so no cast from a
        // bit position to a class number exists to be got wrong.
        let mut index = 0;
        while index < COUNT {
            let class = Class::ALL[index];
            if self.contains(class) {
                return Some(class);
            }
            index = index.saturating_add(1);
        }
        None
    }

    /// The classes in this set, in class-number order. JLReq: §3.9.2
    pub fn classes(self) -> impl Iterator<Item = Class> {
        Class::ALL
            .into_iter()
            .filter(move |class| self.contains(*class))
    }
}

impl Class {
    /// This class's bit in a [`ClassSet`].
    const fn bit(self) -> u32 {
        1_u32 << (self as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::{COUNT, Class, ClassSet};

    #[test]
    fn every_class_is_numbered_as_jlreq_numbers_it() {
        for (position, class) in Class::ALL.into_iter().enumerate() {
            let expected = u8::try_from(position).expect("thirty fits in a byte") + 1;
            assert_eq!(class.number(), expected, "{class:?}");
            assert_eq!(
                Class::from_number(expected),
                Some(class),
                "the numbering round-trips, so a matrix coordinate and a variant are one \
                 thing"
            );
        }
    }

    #[test]
    fn a_number_outside_the_closed_set_names_no_class() {
        assert_eq!(
            Class::from_number(0),
            None,
            "JLReq numbers its classes from one"
        );
        assert_eq!(
            Class::from_number(31),
            None,
            "§3.9.2 closes the set at thirty, which is why this type is exhaustive"
        );
    }

    #[test]
    fn the_identifier_is_the_one_every_rule_sentence_writes() {
        assert_eq!(Class::OpeningBracket.id(), "cl-01");
        assert_eq!(Class::Ideographic.id(), "cl-19");
        assert_eq!(
            Class::InTateChuYoko.id(),
            "cl-30",
            "JLReq zero-pads to two digits and writes `cl-30`, never `cl-3`"
        );
    }

    #[test]
    fn the_names_are_the_ones_section_3_9_2_publishes() {
        assert_eq!(Class::Comma.name_en(), "Commas");
        assert_eq!(Class::Comma.name_ja(), "読点類");
        assert_eq!(
            Class::InOrnamentedComplex.name_en(),
            "Ornamented character complexes",
            "the name is §3.9.2's own, which is what makes it a quotation rather than a \
             paraphrase"
        );
    }

    #[test]
    fn twenty_five_classes_enumerate_their_members_and_five_do_not() {
        let enumerated = Class::ALL
            .into_iter()
            .filter(|class| class.enumeration().is_some())
            .count();
        assert_eq!(enumerated, 25);
        for silent in [
            Class::AsReferenceMark,
            Class::InOrnamentedComplex,
            Class::InNonJukugoRubyComplex,
            Class::InJukugoRubyComplex,
            Class::InTateChuYoko,
        ] {
            assert_eq!(
                silent.enumeration(),
                None,
                "{silent:?}'s section text reads in full \"Any character may participate \
                 in …\""
            );
        }
    }

    #[test]
    fn an_enumerating_class_names_its_own_appendix_section() {
        assert_eq!(
            Class::Ideographic.enumeration(),
            jlreq_spec::Address::parse("A.19"),
            "the address is the specification's own rendered section number, and the              grammar is what holds this crate's spelling and the inventory's equal"
        );
    }

    #[test]
    fn a_set_holds_the_classes_put_into_it_and_no_others() {
        let set = ClassSet::of(Class::Ideographic).with(Class::Western);
        assert!(set.contains(Class::Ideographic) && set.contains(Class::Western));
        assert!(!set.contains(Class::Hiragana));
        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());
    }

    #[test]
    fn a_set_iterates_in_class_number_order() {
        let set = ClassSet::of(Class::Western).with(Class::OpeningBracket);
        assert!(
            set.classes().eq([Class::OpeningBracket, Class::Western]),
            "the order is the specification's numbering and never the insertion order, \
             which is what lets a report be compared byte for byte"
        );
    }

    #[test]
    fn the_full_set_holds_every_class_and_the_empty_one_holds_none() {
        assert_eq!(ClassSet::ALL.len(), COUNT);
        assert!(ClassSet::ALL.classes().eq(Class::ALL));
        assert!(ClassSet::EMPTY.is_empty());
        assert_eq!(ClassSet::EMPTY.classes().count(), 0);
    }

    #[test]
    fn a_class_removed_from_a_set_is_not_in_it() {
        assert_eq!(
            ClassSet::ALL.without(Class::Ideographic).len(),
            COUNT - 1,
            "narrowing a candidate set is removing the classes the caller's facts rule out"
        );
        assert!(
            !ClassSet::ALL
                .without(Class::Ideographic)
                .contains(Class::Ideographic)
        );
    }

    #[test]
    fn a_set_of_one_names_that_one_and_a_wider_set_names_none() {
        assert_eq!(ClassSet::of(Class::Katakana).only(), Some(Class::Katakana));
        assert_eq!(ClassSet::EMPTY.only(), None);
        assert_eq!(
            ClassSet::of(Class::Katakana).with(Class::Hiragana).only(),
            None,
            "two surviving candidates are an ambiguity to report, never a class to pick"
        );
    }

    #[test]
    fn the_five_classes_whose_advance_section_3_1_2_states_are_those_five() {
        let stated: ClassSet = Class::ALL
            .into_iter()
            .filter(|class| class.advance_is_stated_half_width())
            .fold(ClassSet::EMPTY, ClassSet::with);
        assert_eq!(
            stated,
            ClassSet::of(Class::OpeningBracket)
                .with(Class::ClosingBracket)
                .with(Class::MiddleDot)
                .with(Class::FullStop)
                .with(Class::Comma),
            "§3.1.2 names exactly these five, and `Text::new` requires a frame on each"
        );
    }

    #[test]
    fn the_construct_classes_are_the_ones_no_stream_of_items_can_decide() {
        let construct: ClassSet = Class::ALL
            .into_iter()
            .filter(|class| class.is_construct_membership())
            .fold(ClassSet::EMPTY, ClassSet::with);
        assert_eq!(
            construct,
            ClassSet::of(Class::AsReferenceMark)
                .with(Class::InOrnamentedComplex)
                .with(Class::InNonJukugoRubyComplex)
                .with(Class::InJukugoRubyComplex)
                .with(Class::InGroupedNumeral)
                .with(Class::InUnitSymbol)
                .with(Class::WarichuOpeningBracket)
                .with(Class::WarichuClosingBracket)
                .with(Class::InTateChuYoko),
            "the five that enumerate nothing, and the four that enumerate what may appear \
             inside a construct"
        );
        assert!(
            construct
                .classes()
                .all(|class| !class.advance_is_stated_half_width()),
            "no class is both, so requiring a frame and reporting a construct axis never \
             fire over one candidate"
        );
    }
}
