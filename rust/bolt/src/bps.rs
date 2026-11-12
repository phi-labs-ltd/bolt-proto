//! Conversions and `num_traits` impls for [`Bps`].
//!
//! `Bps.value` is a `google.type.Decimal` string in basis points (`"1.2"` is 1.2 bps).
//! A ratio is `bps / 10_000`, so `0.0001` is 1 bps and `1` is 100%.
//!
//! `Bps` is a `String` wrapper, so it can't be `Copy` or build non-empty consts. That rules
//! out `AsPrimitive`, `ConstZero` and `ConstOne`; `FromPrimitive` and `CheckedMul` are here.

use crate::bolt::common::domain_types::v1::Bps;
use crate::google::r#type::Decimal as ProtoDecimal;
use core::fmt;
use core::ops::Mul;
use core::str::FromStr;
use num_traits::{CheckedMul, FromPrimitive};
use rust_decimal::Decimal;

/// Decimal places between bps and a ratio (1 bps = 1e-4).
const RATIO_SHIFT: u32 = 4;

/// Why a [`Bps`] and a [`Decimal`] could not be converted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BpsConversionError {
    /// `Bps.value` is not set.
    Missing,
    /// `Bps.value` is not a valid decimal.
    Invalid(String),
    /// The value is too large for a [`Decimal`].
    Overflow,
    /// The value has too many decimal places for a [`Decimal`] to hold exactly.
    Inexact,
}

impl fmt::Display for BpsConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str("Bps value is not set"),
            Self::Invalid(s) => write!(f, "Bps value {s:?} is not a valid decimal"),
            Self::Overflow => f.write_str("Bps value is too large for a Decimal"),
            Self::Inexact => f.write_str("Bps value has too many decimal places for a Decimal"),
        }
    }
}

impl std::error::Error for BpsConversionError {}

impl Bps {
    /// A `Bps` from a decimal number of basis points.
    pub fn from_bps(bps: Decimal) -> Self {
        Self {
            value: Some(ProtoDecimal {
                value: bps.normalize().to_string(),
            }),
        }
    }

    /// The value as a [`Decimal`] number of basis points.
    pub fn to_bps(&self) -> Result<Decimal, BpsConversionError> {
        let s = &self
            .value
            .as_ref()
            .ok_or(BpsConversionError::Missing)?
            .value;
        Decimal::from_str(s)
            .or_else(|_| Decimal::from_scientific(s))
            .map_err(|_| BpsConversionError::Invalid(s.clone()))
    }
}

/// Converts a ratio (`0.0001` is 1 bps, `1` is 100%).
impl TryFrom<Decimal> for Bps {
    type Error = BpsConversionError;

    fn try_from(ratio: Decimal) -> Result<Self, Self::Error> {
        ratio
            .checked_mul(Decimal::from(10i64.pow(RATIO_SHIFT)))
            .map(Self::from_bps)
            .ok_or(BpsConversionError::Overflow)
    }
}

/// Converts to a ratio (`0.0001` is 1 bps, `1` is 100%). Fallible because the wire value is
/// a string that may be missing or malformed.
impl TryFrom<Bps> for Decimal {
    type Error = BpsConversionError;

    fn try_from(value: Bps) -> Result<Self, Self::Error> {
        let mut ratio = value.to_bps()?;
        ratio
            .set_scale(ratio.scale() + RATIO_SHIFT)
            .map_err(|_| BpsConversionError::Inexact)?;
        Ok(ratio)
    }
}

impl Mul for Bps {
    type Output = Self;

    /// Multiplies as numbers of basis points (`2 bps * 3 bps = 6 bps`). Panics if either
    /// value is missing or malformed, or on overflow; use [`CheckedMul`] to avoid that.
    fn mul(self, rhs: Self) -> Self {
        self.checked_mul(&rhs).expect("Bps mul failed")
    }
}

impl CheckedMul for Bps {
    /// `None` if either value is missing or malformed, or on overflow.
    fn checked_mul(&self, v: &Self) -> Option<Self> {
        let product = self.to_bps().ok()?.checked_mul(v.to_bps().ok()?)?;
        Some(Self::from_bps(product))
    }
}

impl FromPrimitive for Bps {
    /// `n` whole basis points.
    fn from_i64(n: i64) -> Option<Self> {
        Some(Self::from_bps(Decimal::from(n)))
    }

    fn from_u64(n: u64) -> Option<Self> {
        Some(Self::from_bps(Decimal::from(n)))
    }

    /// `None` for NaN, infinity or out of range.
    fn from_f64(n: f64) -> Option<Self> {
        Decimal::from_f64(n).map(Self::from_bps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn bps(s: &str) -> Bps {
        Bps {
            value: Some(ProtoDecimal { value: s.into() }),
        }
    }

    #[test]
    fn ratio_round_trip() {
        let b = Bps::try_from(dec("0.00012")).unwrap();
        assert_eq!(b, bps("1.2"));
        assert_eq!(Decimal::try_from(b).unwrap(), dec("0.00012"));
        assert_eq!(Bps::try_from(dec("-0.0005")).unwrap(), bps("-5"));
        assert_eq!(Bps::try_from(dec("1")).unwrap(), bps("10000"));
    }

    #[test]
    fn errors() {
        assert_eq!(
            Bps::try_from(Decimal::MAX),
            Err(BpsConversionError::Overflow)
        );
        assert_eq!(
            Decimal::try_from(Bps::default()),
            Err(BpsConversionError::Missing)
        );
        assert_eq!(
            Decimal::try_from(bps("abc")),
            Err(BpsConversionError::Invalid("abc".into()))
        );
        assert_eq!(Decimal::try_from(bps("1E1")).unwrap(), dec("0.001"));
        assert_eq!(
            Decimal::try_from(Bps::from_bps(Decimal::new(1, 26))),
            Err(BpsConversionError::Inexact)
        );
    }

    #[test]
    fn mul() {
        assert_eq!(bps("2") * bps("3"), bps("6"));
        assert_eq!(bps("1.5").checked_mul(&bps("2")), Some(bps("3")));
        assert_eq!(bps("x").checked_mul(&bps("2")), None);
        assert_eq!(Bps::default().checked_mul(&bps("2")), None);
    }

    #[test]
    fn primitives() {
        assert_eq!(Bps::from_i64(5), Some(bps("5")));
        assert_eq!(Bps::from_u64(7), Some(bps("7")));
        assert_eq!(Bps::from_f64(1.2), Some(bps("1.2")));
        assert_eq!(Bps::from_f64(f64::NAN), None);
    }
}
