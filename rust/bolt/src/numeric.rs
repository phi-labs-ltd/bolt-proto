use crate::bolt::common::numeric::v3::UInt128;
use crate::bolt::common::numeric::v3::UInt256;
#[cfg(feature = "num")]
use ruint::aliases::U256;
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
        u128::from(self.clone()).cmp(&u128::from(other.clone()))
    }
}

impl PartialOrd for UInt128 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UInt256 {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.limb3, self.limb2, self.limb1, self.limb0).cmp(&(
            other.limb3,
            other.limb2,
            other.limb1,
            other.limb0,
        ))
    }
}

impl PartialOrd for UInt256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "num")]
/// Convert with `.into()`: `U256::from` here is ruint's own function and doesn't compile.
impl From<UInt256> for U256 {
    fn from(value: UInt256) -> Self {
        Self::from_limbs([value.limb0, value.limb1, value.limb2, value.limb3])
    }
}

#[cfg(feature = "num")]
impl From<U256> for UInt256 {
    fn from(value: U256) -> Self {
        let [limb0, limb1, limb2, limb3] = value.into_limbs();
        Self {
            limb3,
            limb2,
            limb1,
            limb0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uint128_keeps_hi_before_lo() {
        let proto = UInt128 { hi: 2, lo: 1 };
        let value: u128 = (2 << 64) + 1;
        assert_eq!(u128::from(proto), value);
        assert_eq!(UInt128::from(value), proto);
    }

    #[cfg(feature = "num")]
    #[test]
    fn uint256_keeps_limb3_most_significant() {
        let proto = UInt256 {
            limb3: 4,
            limb2: 3,
            limb1: 2,
            limb0: 1,
        };
        let value =
            (U256::from(4) << 192) + (U256::from(3) << 128) + (U256::from(2) << 64) + U256::from(1);
        let converted: U256 = proto.into();
        assert_eq!(converted, value);
        assert_eq!(UInt256::from(value), proto);
    }

    #[test]
    fn uint128_orders_numerically() {
        let small = UInt128 {
            hi: 0,
            lo: u64::MAX,
        };
        let big = UInt128 { hi: 1, lo: 0 };
        assert!(small < big && big > small && small <= small);
        assert_eq!(small.cmp(&small), Ordering::Equal);
    }

    #[test]
    fn uint256_orders_numerically() {
        let small = UInt256 {
            limb3: 0,
            limb2: u64::MAX,
            limb1: u64::MAX,
            limb0: u64::MAX,
        };
        let big = UInt256 {
            limb3: 1,
            limb2: 0,
            limb1: 0,
            limb0: 0,
        };
        assert!(small < big && big > small && small <= small);
        assert_eq!(small.cmp(&small), Ordering::Equal);
    }
}
