use bolt_proto::bolt::assets::v1::Pair;
use bolt_proto::bolt::outpost::inventory_feeder::v1::HedgedExposure;

#[test]
fn hedged_exposure_id_is_instrument_base_and_quote() {
    let exposure = HedgedExposure {
        instrument: Some(Pair {
            base: "SUI".into(),
            quote: "USDC".into(),
        }),
        hedged_exposure: None,
    };
    assert_eq!(exposure.id(), Some(("SUI", "USDC")));
}

#[test]
fn hedged_exposure_id_is_none_without_instrument() {
    assert_eq!(HedgedExposure::default().id(), None);
}
