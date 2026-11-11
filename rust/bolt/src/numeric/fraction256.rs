use crate::bolt::common::numeric::v3::Fraction256;
use ruint::aliases::{U256, U512};
use std::cmp::Ordering;

impl Fraction256 {
    /// Compares the rational values, so 1/2 and 2/4 are `Equal`.
    ///
    /// This is a method, not `Ord`/`PartialOrd`: prost derives `PartialEq` structurally
    /// (1/2 != 2/4), and an ordering that disagrees with `==` would break the trait contracts.
    ///
    /// A missing numerator is zero, and zero is never negative whatever `is_negative` says.
    /// Returns `None` if either denominator is missing or zero.
    pub fn cmp_value(&self, other: &Self) -> Option<Ordering> {
        let (a_num, a_den) = parts(self)?;
        let (b_num, b_den) = parts(other)?;
        let a_sign = sign(&a_num, self.is_negative);
        let b_sign = sign(&b_num, other.is_negative);
        if a_sign != b_sign || a_sign == 0 {
            return Some(a_sign.cmp(&b_sign));
        }
        // Denominators are positive: compare a_num*b_den vs b_num*a_den in 512 bits.
        let magnitude = wide(a_num, b_den).cmp(&wide(b_num, a_den));
        Some(if a_sign < 0 {
            magnitude.reverse()
        } else {
            magnitude
        })
    }
}

fn parts(fraction: &Fraction256) -> Option<(U256, U256)> {
    let numerator = fraction.numerator.clone().map_or(U256::ZERO, Into::into);
    let denominator: U256 = fraction.denominator.clone()?.into();
    (!denominator.is_zero()).then_some((numerator, denominator))
}

fn sign(numerator: &U256, is_negative: bool) -> i8 {
    if numerator.is_zero() {
        0
    } else if is_negative {
        -1
    } else {
        1
    }
}

fn wide(a: U256, b: U256) -> U512 {
    U512::from(a) * U512::from(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frac(num: U256, den: U256, is_negative: bool) -> Fraction256 {
        Fraction256 {
            numerator: Some(num.into()),
            denominator: Some(den.into()),
            is_negative,
        }
    }

    fn small(num: u64, den: u64, is_negative: bool) -> Fraction256 {
        frac(U256::from(num), U256::from(den), is_negative)
    }

    #[test]
    fn compares_value_not_representation() {
        assert_eq!(
            small(1, 2, false).cmp_value(&small(2, 4, false)),
            Some(Ordering::Equal)
        );
        assert_eq!(
            small(1, 3, false).cmp_value(&small(1, 2, false)),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn handles_signs_and_zero() {
        assert_eq!(
            small(1, 2, true).cmp_value(&small(1, 3, false)),
            Some(Ordering::Less)
        );
        assert_eq!(
            small(1, 3, true).cmp_value(&small(1, 2, true)),
            Some(Ordering::Greater)
        );
        assert_eq!(
            small(0, 7, true).cmp_value(&small(0, 9, false)),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn invalid_denominator_is_none() {
        assert_eq!(small(1, 0, false).cmp_value(&small(1, 2, false)), None);
    }

    #[test]
    fn products_beyond_256_bits_do_not_overflow() {
        let max = U256::MAX;
        let one = U256::from(1);
        assert_eq!(
            frac(max - one, max, false).cmp_value(&frac(max, max - one, false)),
            Some(Ordering::Less)
        );
    }
}
