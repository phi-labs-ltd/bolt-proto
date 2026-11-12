//! `Bps.value` is a decimal string in basis points. A ratio is `bps / 10_000` (`0.0001` is 1 bps).

use crate::bolt::common::domain_types::v1::Bps;
use crate::google::r#type::Decimal as ProtoDecimal;
use core::fmt;
use core::ops::{Add, Mul};
use core::str::FromStr;
use num_traits::{CheckedMul, One, Zero};
use rust_decimal::Decimal;

const RATIO_SHIFT: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BpsConversionError {
    Missing,
    Invalid(String),
    Overflow,
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
    fn from_bps(bps: Decimal) -> Self {
        Self {
            value: Some(ProtoDecimal {
                value: bps.normalize().to_string(),
            }),
        }
    }

    fn to_bps(&self) -> Result<Decimal, BpsConversionError> {
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

impl TryFrom<Decimal> for Bps {
    type Error = BpsConversionError;

    fn try_from(ratio: Decimal) -> Result<Self, Self::Error> {
        ratio
            .checked_mul(Decimal::from(10i64.pow(RATIO_SHIFT)))
            .map(Self::from_bps)
            .ok_or(BpsConversionError::Overflow)
    }
}

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

impl Add for Bps {
    type Output = Self;

    /// Panics if either value is missing or malformed, or on overflow.
    fn add(self, rhs: Self) -> Self {
        let sum = self.to_bps().and_then(|a| {
            let b = rhs.to_bps()?;
            a.checked_add(b).ok_or(BpsConversionError::Overflow)
        });
        Self::from_bps(sum.expect("Bps add failed"))
    }
}

impl Zero for Bps {
    fn zero() -> Self {
        Self::from_bps(Decimal::ZERO)
    }

    fn is_zero(&self) -> bool {
        self.to_bps().is_ok_and(|v| v.is_zero())
    }
}

impl One for Bps {
    fn one() -> Self {
        Self::from_bps(Decimal::ONE)
    }

    fn is_one(&self) -> bool {
        self.to_bps().is_ok_and(|v| v.is_one())
    }
}

impl Mul for Bps {
    type Output = Self;

    /// Panics if either value is missing or malformed, or on overflow; use `checked_mul` instead.
    fn mul(self, rhs: Self) -> Self {
        self.checked_mul(&rhs).expect("Bps mul failed")
    }
}

impl CheckedMul for Bps {
    fn checked_mul(&self, v: &Self) -> Option<Self> {
        let product = self.to_bps().ok()?.checked_mul(v.to_bps().ok()?)?;
        Some(Self::from_bps(product))
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
    fn zero_and_one() {
        assert!(Bps::zero().is_zero());
        assert!(bps("0.00").is_zero());
        assert!(!Bps::default().is_zero());
        assert!(Bps::one().is_one());
        assert!(bps("1.0").is_one());
        assert!(!bps("x").is_one());
        let x = bps("1.2");
        assert_eq!(x.clone() * Bps::one(), x);
        assert_eq!(x.clone() + Bps::zero(), x);
        assert_eq!(bps("1.5") + bps("2"), bps("3.5"));
    }
}
