// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{LayoutError, OptionKind};

pub(crate) const SUBPIXELS_PER_UNIT: f32 = 64.0;
// Kept below the first f32 integer precision cliff after 26.6 scaling, and well below the
// i32 geometry overflow boundary used by the composer.
const MAX_ABS_UNITS: f32 = 8_388_607.0;

pub(crate) fn positive(value: f32, option: OptionKind) -> Result<i32, LayoutError> {
    if !value.is_finite() || value <= 0.0 || value > MAX_ABS_UNITS {
        return Err(LayoutError::invalid_option(
            option,
            "value must be finite, positive, and representable in 26.6 fixed point",
        ));
    }
    Ok(quantize(value))
}

pub(crate) fn non_negative(value: f32, option: OptionKind) -> Result<i32, LayoutError> {
    if !value.is_finite() || value < 0.0 || value > MAX_ABS_UNITS {
        return Err(LayoutError::invalid_option(
            option,
            "value must be finite, non-negative, and representable in 26.6 fixed point",
        ));
    }
    Ok(quantize(value))
}

pub(crate) fn finite(value: f32, option: OptionKind) -> Result<f32, LayoutError> {
    if !value.is_finite() || value.abs() > MAX_ABS_UNITS {
        return Err(LayoutError::invalid_option(
            option,
            "value must be finite and representable",
        ));
    }
    Ok(to_f32(quantize(value)))
}

pub(crate) fn quantize(value: f32) -> i32 {
    rounded_f32_to_i32((value * SUBPIXELS_PER_UNIT).round())
}

pub(crate) fn to_f32(value: i32) -> f32 {
    // Split at a power of two so both integer-to-float conversions use lossless `From`
    // implementations and the final f32 rounding is platform-independent.
    let high = i16::try_from(value.div_euclid(65_536)).unwrap_or_else(|_| {
        if value.is_negative() {
            i16::MIN
        } else {
            i16::MAX
        }
    });
    let low = u16::try_from(value.rem_euclid(65_536)).unwrap_or_default();
    f32::from(high).mul_add(65_536.0, f32::from(low)) / SUBPIXELS_PER_UNIT
}

fn rounded_f32_to_i32(value: f32) -> i32 {
    if value == 0.0 {
        return 0;
    }
    let bits = value.to_bits();
    let negative = bits & 0x8000_0000 != 0;
    let exponent = (bits >> 23) & 0xff;
    if exponent < 127 {
        return 0;
    }
    // The stored mantissa and the implicit leading bit are disjoint.  Express the
    // reconstruction without a bitwise `or`, whose `xor` mutation is equivalent
    // for this mask and therefore cannot be distinguished by any input.
    let significand = 0x0080_0000_u32.saturating_add(bits & 0x007f_ffff);
    let magnitude = if exponent >= 150 {
        significand
            .checked_shl(exponent.saturating_sub(150))
            .unwrap_or(u32::MAX)
    } else {
        significand >> 150_u32.saturating_sub(exponent)
    };
    let signed = i32::try_from(magnitude).unwrap_or(i32::MAX);
    if negative {
        signed.saturating_neg()
    } else {
        signed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OptionKind;

    #[test]
    fn fixed_point_conversion_rounds_to_the_nearest_subpixel() {
        assert_eq!(quantize(1.0), 64);
        assert_eq!(quantize(-1.0), -64);
        assert_eq!(quantize(1.007_812_5), 65);
        assert_eq!(quantize(to_f32(65)), 65);
        assert_eq!(quantize(to_f32(-65)), -65);
        assert_eq!(rounded_f32_to_i32(1.0), 1);
        assert_eq!(rounded_f32_to_i32(-1.0), -1);
    }

    #[test]
    fn validated_numeric_domains_include_their_exact_finite_ceiling() {
        assert!(positive(MAX_ABS_UNITS, OptionKind::FontSize).is_ok());
        assert!(positive(MAX_ABS_UNITS + 1.0, OptionKind::FontSize).is_err());
        assert!(positive(0.0, OptionKind::FontSize).is_err());
        assert!(positive(f32::NAN, OptionKind::FontSize).is_err());

        assert!(non_negative(MAX_ABS_UNITS, OptionKind::LineGap).is_ok());
        assert!(non_negative(MAX_ABS_UNITS + 1.0, OptionKind::LineGap).is_err());
        assert!(non_negative(0.0, OptionKind::LineGap).is_ok());
        assert!(non_negative(-1.0, OptionKind::LineGap).is_err());

        assert!(finite(MAX_ABS_UNITS, OptionKind::Variation).is_ok());
        assert!(finite(-MAX_ABS_UNITS, OptionKind::Variation).is_ok());
        assert!(finite(MAX_ABS_UNITS + 1.0, OptionKind::Variation).is_err());
        assert!(finite(-(MAX_ABS_UNITS + 1.0), OptionKind::Variation).is_err());
        assert!(finite(f32::INFINITY, OptionKind::Variation).is_err());
    }
}
