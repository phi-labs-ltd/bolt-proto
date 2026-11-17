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
fn pair_validate() {
    let cases: &[(&str, &str, Result<(), &str>)] = &[
        ("SUI", "USDC", Ok(())),
        (
            "",
            "USDC",
            Err(
                r#"bolt.assets.v1.Pair.base: failed `!base.is_empty()`; self = Pair { base: "", quote: "USDC" }"#,
            ),
        ),
        (
            "SUI",
            "",
            Err(
                r#"bolt.assets.v1.Pair.quote: failed `!quote.is_empty()`; self = Pair { base: "SUI", quote: "" }"#,
            ),
        ),
        (
            "",
            "",
            Err(
                r#"bolt.assets.v1.Pair.base: failed `!base.is_empty()`; self = Pair { base: "", quote: "" }"#,
            ),
        ),
    ];
    for (base, quote, want) in cases {
        let pair = Pair {
            base: (*base).into(),
            quote: (*quote).into(),
        };
        assert_eq!(
            pair.validate(),
            want.map_err(String::from),
            "base={base:?} quote={quote:?}"
        );
    }
}
