use crate::bolt::common::numeric::v3::{UInt128, UInt256};
use ruint::aliases::U256;

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

#[cfg(feature = "num")]
/// Convert with `.into()`: `U256::from` here is ruint's own function and doesn't compile.
impl From<UInt256> for U256 {
    fn from(value: UInt256) -> Self {
        Self::from_limbs([value.limb0, value.limb1, value.limb2, value.limb3])
    }
}

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
}
