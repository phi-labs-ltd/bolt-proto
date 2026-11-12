//! Conversions and `num_traits` impls for [`Bps`].
//!
//! The wire value is `milli_bps`: thousandths of a basis point, so `1.2 bps` is `1200`.
//! As a ratio, `1 bps = 0.0001`. The `num_traits` impls treat a `Bps` as a number of
//! whole basis points (`ONE` is 1 bps, `from_i64(5)` is 5 bps); the milli scale is hidden.

use crate::bolt::common::domain_types::v1::Bps;
use core::fmt;
use core::ops::{Add, Mul};
use num_traits::{AsPrimitive, CheckedMul, ConstOne, ConstZero, FromPrimitive, One, Zero};
use rust_decimal::Decimal;

/// Raw units per basis point.
const MILLI_PER_BPS: i64 = 1_000;
/// Decimal scale of a ratio in milli-bps: 1 milli-bps = 1e-7.
const RATIO_SCALE: u32 = 7;

/// Why a [`Decimal`] ratio could not become a [`Bps`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpsConversionError {
    /// The ratio has finer precision than 1/1000 of a basis point (1e-7).
    Inexact,
    /// The ratio is too large to fit in `milli_bps`.
    Overflow,
}

impl fmt::Display for BpsConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inexact => f.write_str("ratio is finer than 1/1000 of a basis point"),
            Self::Overflow => f.write_str("ratio does not fit in a Bps"),
        }
    }
}

impl std::error::Error for BpsConversionError {}

impl Bps {
    /// A `Bps` from raw thousandths of a basis point.
    pub const fn from_milli_bps(milli_bps: i64) -> Self {
        Self { milli_bps }
    }
}

/// Converts a ratio (`0.0001` is 1 bps, `1` is 100%). Fails rather than rounding.
impl TryFrom<Decimal> for Bps {
    type Error = BpsConversionError;

    fn try_from(ratio: Decimal) -> Result<Self, Self::Error> {
        let scaled = ratio
            .checked_mul(Decimal::from(10i64.pow(RATIO_SCALE)))
            .ok_or(BpsConversionError::Overflow)?;
        if !scaled.fract().is_zero() {
            return Err(BpsConversionError::Inexact);
        }
        i64::try_from(scaled.trunc())
            .map(Self::from_milli_bps)
            .map_err(|_| BpsConversionError::Overflow)
    }
}

/// Converts to a ratio (`0.0001` is 1 bps, `1` is 100%). Exact.
impl From<Bps> for Decimal {
    fn from(value: Bps) -> Self {
        Decimal::new(value.milli_bps, RATIO_SCALE)
    }
}

impl Add for Bps {
    type Output = Self;

    /// Panics on overflow, like integer `+` in debug builds.
    fn add(self, rhs: Self) -> Self {
        Self::from_milli_bps(
            self.milli_bps
                .checked_add(rhs.milli_bps)
                .expect("Bps add overflow"),
        )
    }
}

impl Mul for Bps {
    type Output = Self;

    /// Multiplies as numbers of basis points (`2 bps * 3 bps = 6 bps`), truncating toward
    /// zero below 1/1000 bps. Panics on overflow.
    fn mul(self, rhs: Self) -> Self {
        self.checked_mul(&rhs).expect("Bps mul overflow")
    }
}

impl CheckedMul for Bps {
    fn checked_mul(&self, v: &Self) -> Option<Self> {
        let product =
            i128::from(self.milli_bps) * i128::from(v.milli_bps) / i128::from(MILLI_PER_BPS);
        i64::try_from(product).ok().map(Self::from_milli_bps)
    }
}

impl Zero for Bps {
    fn zero() -> Self {
        Self::ZERO
    }

    fn is_zero(&self) -> bool {
        self.milli_bps == 0
    }
}

impl One for Bps {
    fn one() -> Self {
        Self::ONE
    }
}

impl ConstZero for Bps {
    const ZERO: Self = Self::from_milli_bps(0);
}

impl ConstOne for Bps {
    const ONE: Self = Self::from_milli_bps(MILLI_PER_BPS);
}

impl FromPrimitive for Bps {
    /// `n` whole basis points.
    fn from_i64(n: i64) -> Option<Self> {
        n.checked_mul(MILLI_PER_BPS).map(Self::from_milli_bps)
    }

    fn from_u64(n: u64) -> Option<Self> {
        i64::try_from(n).ok().and_then(Self::from_i64)
    }

    /// Rounds to the nearest 1/1000 bps; `None` for NaN, infinity or out of range.
    fn from_f64(n: f64) -> Option<Self> {
        let milli = (n * MILLI_PER_BPS as f64).round();
        // 2^63 is exactly representable; the upper bound is exclusive.
        (milli.is_finite() && milli >= -(2f64.powi(63)) && milli < 2f64.powi(63))
            .then(|| Self::from_milli_bps(milli as i64))
    }
}

macro_rules! as_primitive_int {
    ($($t:ty),*) => {$(
        /// Whole basis points, truncating toward zero.
        impl AsPrimitive<$t> for Bps {
            fn as_(self) -> $t {
                (self.milli_bps / MILLI_PER_BPS) as $t
            }
        }
    )*};
}

macro_rules! as_primitive_float {
    ($($t:ty),*) => {$(
        /// Basis points including the fractional part.
        impl AsPrimitive<$t> for Bps {
            fn as_(self) -> $t {
                self.milli_bps as $t / MILLI_PER_BPS as $t
            }
        }
    )*};
}

as_primitive_int!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
as_primitive_float!(f32, f64);

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn ratio_round_trip() {
        let bps = Bps::try_from(dec("0.00012")).unwrap();
        assert_eq!(bps.milli_bps, 1200);
        assert_eq!(Decimal::from(bps), dec("0.00012"));
        assert_eq!(Bps::try_from(dec("-0.0005")).unwrap().milli_bps, -5000);
    }

    #[test]
    fn ratio_errors() {
        assert_eq!(
            Bps::try_from(dec("0.00000001")),
            Err(BpsConversionError::Inexact)
        );
        assert_eq!(
            Bps::try_from(Decimal::MAX),
            Err(BpsConversionError::Overflow)
        );
    }

    #[test]
    fn identities() {
        let x = Bps::from_milli_bps(1200);
        assert_eq!(x * Bps::ONE, x);
        assert_eq!(x + Bps::ZERO, x);
        assert!(Bps::zero().is_zero());
        assert_eq!(
            Bps::from_milli_bps(2000) * Bps::from_milli_bps(3000),
            Bps::from_milli_bps(6000)
        );
        assert_eq!(
            Bps::from_milli_bps(i64::MAX).checked_mul(&Bps::from_milli_bps(i64::MAX)),
            None
        );
    }

    #[test]
    fn primitives() {
        assert_eq!(Bps::from_i64(5), Some(Bps::from_milli_bps(5000)));
        assert_eq!(Bps::from_u64(u64::MAX), None);
        assert_eq!(Bps::from_f64(1.2), Some(Bps::from_milli_bps(1200)));
        assert_eq!(Bps::from_f64(f64::NAN), None);
        let x = Bps::from_milli_bps(1200);
        assert_eq!(AsPrimitive::<i32>::as_(x), 1);
        assert_eq!(AsPrimitive::<f64>::as_(x), 1.2);
    }
}
