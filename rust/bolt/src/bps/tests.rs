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
fn zero_and_one() {
    assert_eq!(Bps::zero(), bps("0"));
    assert_eq!(Bps::one(), bps("1"));
    assert_eq!(Decimal::try_from(Bps::one()).unwrap(), dec("0.0001"));
}
