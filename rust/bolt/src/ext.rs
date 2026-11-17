macro_rules! impl_pair {
    (@ty string) => { &str };
    (@ty bytes) => { &[u8] };
    (@ty double) => { f64 };
    (@ty float) => { f32 };
    (@ty int32) => { i32 };
    (@ty sint32) => { i32 };
    (@ty sfixed32) => { i32 };
    (@ty int64) => { i64 };
    (@ty sint64) => { i64 };
    (@ty sfixed64) => { i64 };
    (@ty uint32) => { u32 };
    (@ty fixed32) => { u32 };
    (@ty uint64) => { u64 };
    (@ty fixed64) => { u64 };
    (@ty bool) => { bool };
    (@get string req, $e:expr) => { &*$e };
    (@get string opt, $e:expr) => { $e.as_deref()? };
    (@get bytes req, $e:expr) => { &*$e };
    (@get bytes opt, $e:expr) => { $e.as_deref()? };
    (@get $k:ident req, $e:expr) => { $e };
    (@get $k:ident opt, $e:expr) => { $e? };
    (
        $ty:path;
        paths = $base_path:literal, $quote_path:literal;
        fallible;
        base: [$($bh:ident)*] $bl:ident: $bk:ident $bp:ident;
        quote: [$($qh:ident)*] $ql:ident: $qk:ident $qp:ident;
    ) => {
        impl $ty {
            #[doc = concat!(" (base, quote) from `(bolt.options.v1.pair)`: `", $base_path, "`, `", $quote_path, "`.")]
            /// `None` when a message field or `optional` field on either path is unset.
            pub fn pair(&self) -> ::core::option::Option<(
                $crate::ext::impl_pair!(@ty $bk),
                $crate::ext::impl_pair!(@ty $qk),
            )> {
                ::core::option::Option::Some((
                    $crate::ext::impl_pair!(@get $bk $bp, self$(.$bh.as_ref()?)*.$bl),
                    $crate::ext::impl_pair!(@get $qk $qp, self$(.$qh.as_ref()?)*.$ql),
                ))
            }
        }
    };
    (
        $ty:path;
        paths = $base_path:literal, $quote_path:literal;
        infallible;
        base: [] $bl:ident: $bk:ident req;
        quote: [] $ql:ident: $qk:ident req;
    ) => {
        impl $ty {
            #[doc = concat!(" (base, quote) from `(bolt.options.v1.pair)`: `", $base_path, "`, `", $quote_path, "`.")]
            pub fn pair(&self) -> ($crate::ext::impl_pair!(@ty $bk), $crate::ext::impl_pair!(@ty $qk)) {
                (
                    $crate::ext::impl_pair!(@get $bk req, self.$bl),
                    $crate::ext::impl_pair!(@get $qk req, self.$ql),
                )
            }
        }
    };
}

// `self` comes in from the caller: macro_rules hygiene would otherwise keep rules that
// mention `self` from resolving against the receiver declared here.
macro_rules! impl_validate {
    (@bind val, $e:expr) => { $e };
    (@bind ref, $e:expr) => { &$e };
    (
        $ty:path, $name:literal, $self:tt;
        $($field:ident $bind:ident $proto:literal { $($src:literal => $rule:expr),+ $(,)? })*
    ) => {
        impl $ty {
            /// Checks the `(bolt.options.v1.validate)` rules on each field, in field order,
            /// and returns the first that fails.
            pub fn validate(&$self) -> ::core::result::Result<(), ::prost::alloc::string::String> {
                $({
                    let $field = $crate::ext::impl_validate!(@bind $bind, $self.$field);
                    $(
                        if !($rule) {
                            return ::core::result::Result::Err(::prost::alloc::format!(
                                "{}{:?}",
                                concat!($name, ".", $proto, ": failed `", $src, "`; self = "),
                                $self,
                            ));
                        }
                    )+
                })*
                ::core::result::Result::Ok(())
            }
        }
    };
}

pub(crate) use {impl_pair, impl_validate};

#[cfg(test)]
mod tests {
    use prost::bytes::Bytes;

    #[derive(Debug, Default)]
    struct Leg {
        asset: String,
    }

    #[derive(Debug, Default)]
    struct Nested {
        leg: Option<Leg>,
    }

    #[derive(Debug, Default)]
    struct Flat {
        base: String,
        quote: u64,
    }

    #[derive(Debug, Default)]
    struct Optionals {
        r#type: Option<String>,
        quote: Option<Bytes>,
        count: u32,
        min: u32,
    }

    #[derive(Debug, Default)]
    struct Deep {
        outer: Option<Nested>,
        quote: Option<i64>,
    }

    impl_pair! {
        Flat;
        paths = "base", "quote";
        infallible;
        base: [] base: string req;
        quote: [] quote: uint64 req;
    }

    impl_pair! {
        Optionals;
        paths = "type", "quote";
        fallible;
        base: [] r#type: string opt;
        quote: [] quote: bytes opt;
    }

    impl_pair! {
        Deep;
        paths = "outer.leg.asset", "quote";
        fallible;
        base: [outer leg] asset: string req;
        quote: [] quote: sfixed64 opt;
    }

    impl_validate! {
        Optionals, "test.Optionals", self;
        r#type ref "type" { "r#type.as_deref() != Some(\"\")" => r#type.as_deref() != Some("") }
        count val "count" {
            "count > 0" => count > 0,
            "count >= self.min" => count >= self.min,
        }
    }

    #[test]
    fn infallible_pair() {
        let flat = Flat {
            base: "SUI".into(),
            quote: 7,
        };
        assert_eq!(flat.pair(), ("SUI", 7));
    }

    #[test]
    fn optional_leaves() {
        let full = Optionals {
            r#type: Some("SUI".into()),
            quote: Some(Bytes::from_static(b"USDC")),
            ..Default::default()
        };
        assert_eq!(full.pair(), Some(("SUI", &b"USDC"[..])));
        assert_eq!(Optionals::default().pair(), None);
    }

    #[test]
    fn nested_hops() {
        let deep = Deep {
            outer: Some(Nested {
                leg: Some(Leg {
                    asset: "SUI".into(),
                }),
            }),
            quote: Some(-1),
        };
        assert_eq!(deep.pair(), Some(("SUI", -1)));
        let no_leg = Deep {
            outer: Some(Nested::default()),
            quote: Some(-1),
        };
        assert_eq!(no_leg.pair(), None);
    }

    #[test]
    fn validate_rules_in_order() {
        let ok = Optionals {
            count: 2,
            min: 1,
            ..Default::default()
        };
        assert_eq!(ok.validate(), Ok(()));

        let empty_type = Optionals {
            r#type: Some(String::new()),
            ..ok
        };
        assert_eq!(
            empty_type.validate(),
            Err(r#"test.Optionals.type: failed `r#type.as_deref() != Some("")`; self = Optionals { type: Some(""), quote: None, count: 2, min: 1 }"#.into())
        );

        let zero = Optionals {
            count: 0,
            ..Default::default()
        };
        assert_eq!(
            zero.validate(),
            Err("test.Optionals.count: failed `count > 0`; self = Optionals { type: None, quote: None, count: 0, min: 0 }".into())
        );

        let below_min = Optionals {
            count: 1,
            min: 3,
            ..Default::default()
        };
        assert_eq!(
            below_min.validate(),
            Err("test.Optionals.count: failed `count >= self.min`; self = Optionals { type: None, quote: None, count: 1, min: 3 }".into())
        );
    }
}
