//! `Bps.value` is a decimal string in basis points. A ratio is `bps / 10_000` (`0.0001` is 1 bps).

use crate::bolt::common::domain_types::v1::Bps;
use crate::google::r#type::Decimal as ProtoDecimal;
use core::fmt;
use core::str::FromStr;
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
    pub fn zero() -> Self {
        Self::from_bps(Decimal::ZERO)
    }

    pub fn one() -> Self {
        Self::from_bps(Decimal::ONE)
    }

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

#[cfg(test)]
mod tests;
