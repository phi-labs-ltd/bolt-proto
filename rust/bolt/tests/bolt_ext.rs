use bolt_proto::bolt::assets::v1::Pair;
use bolt_proto::bolt::outpost::inventory_feeder::v1::HedgedExposure;

#[test]
fn hedged_exposure_pair_is_instrument_base_and_quote() {
    let exposure = HedgedExposure {
        instrument: Some(Pair {
            base: "SUI".into(),
            quote: "USDC".into(),
        }),
        hedged_exposure: None,
    };
    assert_eq!(exposure.pair(), Some(("SUI", "USDC")));
}

#[test]
fn hedged_exposure_pair_is_none_without_instrument() {
    assert_eq!(HedgedExposure::default().pair(), None);
}

#[test]
fn pair_validate_accepts_base_and_quote() {
    let pair = Pair {
        base: "SUI".into(),
        quote: "USDC".into(),
    };
    assert_eq!(pair.validate(), Ok(()));
}

#[test]
fn pair_validate_rejects_empty_base_with_self() {
    let pair = Pair {
        base: "".into(),
        quote: "USDC".into(),
    };
    assert_eq!(
        pair.validate(),
        Err(r#"bolt.assets.v1.Pair.base: failed `!base.is_empty()`; self = Pair { base: "", quote: "USDC" }"#.into())
    );
}

#[test]
fn pair_validate_rejects_empty_quote() {
    let pair = Pair {
        base: "SUI".into(),
        quote: "".into(),
    };
    assert!(pair
        .validate()
        .unwrap_err()
        .starts_with("bolt.assets.v1.Pair.quote: failed `!quote.is_empty()`"));
}
