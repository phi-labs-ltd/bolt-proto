use crate::bolt::common::numeric::v3::UInt128;
use std::cmp::Ordering;

impl From<UInt128> for u128 {
    fn from(value: UInt128) -> Self {
        (u128::from(value.hi) << 64) | u128::from(value.lo)
    }
}

impl From<u128> for UInt128 {
    fn from(value: u128) -> Self {
        Self {
            hi: (value >> 64) as u64,
            lo: value as u64,
        }
    }
}

// Eq/PartialEq are derived by prost; ordering is numeric (most significant limb first),
// not the field-declaration order a derive would use.
impl Ord for UInt128 {
    fn cmp(&self, other: &Self) -> Ordering {
        u128::from(*self).cmp(&u128::from(*other))
    }
}

impl PartialOrd for UInt128 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_hi_before_lo() {
        let proto = UInt128 { hi: 2, lo: 1 };
        let value: u128 = (2 << 64) + 1;
        assert_eq!(u128::from(proto), value);
        assert_eq!(UInt128::from(value), proto);
    }

    #[test]
    fn orders_numerically() {
        let small = UInt128 {
            hi: 0,
            lo: u64::MAX,
        };
        let big = UInt128 { hi: 1, lo: 0 };
        assert!(small < big && big > small && small <= small);
        assert_eq!(small.cmp(&small), Ordering::Equal);
    }
}
