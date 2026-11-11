use crate::bolt::common::numeric::v3::Fraction128;
use std::cmp::Ordering;

impl Fraction128 {
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
        let a_sign = sign(a_num, self.is_negative);
        let b_sign = sign(b_num, other.is_negative);
        if a_sign != b_sign || a_sign == 0 {
            return Some(a_sign.cmp(&b_sign));
        }
        // a_num/a_den vs b_num/b_den, denominators positive: compare a_num*b_den vs b_num*a_den.
        let magnitude = mul_wide(a_num, b_den).cmp(&mul_wide(b_num, a_den));
        Some(if a_sign < 0 {
            magnitude.reverse()
        } else {
            magnitude
        })
    }
}

fn parts(fraction: &Fraction128) -> Option<(u128, u128)> {
    let numerator = fraction.numerator.map_or(0, u128::from);
    let denominator = u128::from(fraction.denominator?);
    (denominator != 0).then_some((numerator, denominator))
}

fn sign(numerator: u128, is_negative: bool) -> i8 {
    match (numerator, is_negative) {
        (0, _) => 0,
        (_, true) => -1,
        (_, false) => 1,
    }
}

/// Full 256-bit product as (high, low) halves; tuples compare numerically.
fn mul_wide(a: u128, b: u128) -> (u128, u128) {
    const MASK: u128 = u64::MAX as u128;
    let (a1, a0) = (a >> 64, a & MASK);
    let (b1, b0) = (b >> 64, b & MASK);
    let (ll, lh, hl, hh) = (a0 * b0, a0 * b1, a1 * b0, a1 * b1);
    let mid = (ll >> 64) + (lh & MASK) + (hl & MASK);
    let lo = (ll & MASK) | (mid << 64);
    let hi = hh + (lh >> 64) + (hl >> 64) + (mid >> 64);
    (hi, lo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bolt::common::numeric::v3::UInt128;

    fn frac(num: u128, den: u128, is_negative: bool) -> Fraction128 {
        Fraction128 {
            numerator: Some(UInt128::from(num)),
            denominator: Some(UInt128::from(den)),
            is_negative,
        }
    }

    fn cmp(a: &Fraction128, b: &Fraction128) -> Option<Ordering> {
        a.cmp_value(b)
    }

    #[test]
    fn compares_value_not_representation() {
        assert_eq!(
            cmp(&frac(1, 2, false), &frac(2, 4, false)),
            Some(Ordering::Equal)
        );
        assert_eq!(
            cmp(&frac(1, 3, false), &frac(1, 2, false)),
            Some(Ordering::Less)
        );
        assert_eq!(
            cmp(&frac(3, 2, false), &frac(1, 1, false)),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn handles_signs() {
        assert_eq!(
            cmp(&frac(1, 2, true), &frac(1, 3, false)),
            Some(Ordering::Less)
        );
        assert_eq!(
            cmp(&frac(1, 2, true), &frac(1, 3, true)),
            Some(Ordering::Less)
        );
        assert_eq!(
            cmp(&frac(1, 3, true), &frac(1, 2, true)),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn zero_ignores_sign_and_unset_numerator() {
        let unset = Fraction128 {
            numerator: None,
            denominator: Some(UInt128::from(5u128)),
            is_negative: false,
        };
        assert_eq!(cmp(&frac(0, 7, true), &unset), Some(Ordering::Equal));
        assert_eq!(cmp(&unset, &frac(1, 9, true)), Some(Ordering::Greater));
    }

    #[test]
    fn invalid_denominator_is_none() {
        let unset = Fraction128 {
            numerator: Some(UInt128::from(1u128)),
            denominator: None,
            is_negative: false,
        };
        assert_eq!(cmp(&frac(1, 0, false), &frac(1, 2, false)), None);
        assert_eq!(cmp(&frac(1, 2, false), &unset), None);
    }

    #[test]
    fn products_beyond_128_bits_do_not_overflow() {
        let max = u128::MAX;
        // (max-1)/max < max/(max-1)
        assert_eq!(
            cmp(&frac(max - 1, max, false), &frac(max, max - 1, false)),
            Some(Ordering::Less)
        );
        assert_eq!(
            cmp(&frac(max, max, false), &frac(max - 1, max - 1, false)),
            Some(Ordering::Equal)
        );
    }
}
