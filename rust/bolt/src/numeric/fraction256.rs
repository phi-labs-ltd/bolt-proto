use crate::bolt::common::numeric::v3::Fraction256;
use ruint::aliases::{U256, U512};
use std::cmp::Ordering;

impl Fraction256 {
    /// Compares the rational values, so 1/2 and 2/4 are `Equal`.
    ///
    /// Unlike `Ord`, this ignores representation, so it is not consistent with `==`
    /// (prost derives `PartialEq` structurally, so 1/2 != 2/4).
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

/// Orders by rational value, so 1/3 < 1/2. Fractions of equal value (1/2 and 2/4) are
/// then ordered by their fields, which keeps `cmp` consistent with the derived `==`.
/// Fractions with a missing or zero denominator are invalid and sort before every valid one.
/// Use `cmp_value` to compare values alone.
impl Ord for Fraction256 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_value(other)
            .unwrap_or_else(|| parts(self).is_some().cmp(&parts(other).is_some()))
            .then_with(|| {
                (self.numerator, self.denominator, self.is_negative).cmp(&(
                    other.numerator,
                    other.denominator,
                    other.is_negative,
                ))
            })
    }
}

impl PartialOrd for Fraction256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn parts(fraction: &Fraction256) -> Option<(U256, U256)> {
    let numerator = fraction.numerator.map_or(U256::ZERO, Into::into);
    let denominator: U256 = fraction.denominator?.into();
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

    #[test]
    fn ord_is_consistent_with_eq() {
        let half = small(1, 2, false);
        let two_fourths = small(2, 4, false);
        assert_ne!(half, two_fourths);
        assert_ne!(half.cmp(&two_fourths), Ordering::Equal);
        assert_eq!(half.cmp(&two_fourths), two_fourths.cmp(&half).reverse());
        assert_eq!(half.cmp(&half), Ordering::Equal);
        assert!(small(1, 3, false) < half && half < small(2, 3, false));
        assert!(small(5, 1, true) < small(0, 1, false));
    }

    #[test]
    fn ord_puts_invalid_denominators_first() {
        let zero_den = small(1, 0, false);
        let negative = small(9, 1, true);
        assert!(zero_den < negative);
        let mut all = vec![small(1, 2, false), zero_den, negative];
        all.sort();
        assert_eq!(all, vec![zero_den, negative, small(1, 2, false)]);
    }
}
