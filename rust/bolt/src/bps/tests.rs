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
