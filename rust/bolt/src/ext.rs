macro_rules! impl_pair {
    (@get req, $e:expr) => { &*$e };
    (@get opt, $e:expr) => { $e.as_deref()? };
    (
        $ty:path;
        paths = $base_path:literal, $quote_path:literal;
        fallible;
        base: [$($bh:ident)*] $bl:ident $bp:ident;
        quote: [$($qh:ident)*] $ql:ident $qp:ident;
    ) => {
        impl $ty {
            #[doc = concat!(" (base, quote) from `(bolt.options.v1.pair)`: `", $base_path, "`, `", $quote_path, "`.")]
            /// `None` when a message field or `optional` field on either path is unset.
            pub fn pair(&self) -> ::core::option::Option<(&str, &str)> {
                ::core::option::Option::Some((
                    $crate::ext::impl_pair!(@get $bp, self$(.$bh.as_ref()?)*.$bl),
                    $crate::ext::impl_pair!(@get $qp, self$(.$qh.as_ref()?)*.$ql),
                ))
            }
        }
    };
    (
        $ty:path;
        paths = $base_path:literal, $quote_path:literal;
        infallible;
        base: [] $bl:ident req;
        quote: [] $ql:ident req;
    ) => {
        impl $ty {
            #[doc = concat!(" (base, quote) from `(bolt.options.v1.pair)`: `", $base_path, "`, `", $quote_path, "`.")]
            pub fn pair(&self) -> (&str, &str) {
                (&*self.$bl, &*self.$ql)
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

/// The encoded file descriptor sets a gRPC reflection service needs to resolve a generated
/// server's service: its own package and every package and well-known type that package
/// imports, transitively. Each entry is a `FileDescriptorSet`, as taken by
/// `tonic_reflection::server::Builder::register_encoded_file_descriptor_set`.
pub trait ReflectionFiles {
    const FILE_DESCRIPTOR_SETS: &'static [&'static [u8]];
}

macro_rules! impl_reflection_files {
    (
        $test:ident, $service:literal;
        $($server:ident)::+;
        $($descriptor_set:path),+ $(,)?
    ) => {
        #[cfg(feature = "grpc")]
        impl<T> $crate::ReflectionFiles for $crate::$($server)::+<T> {
            const FILE_DESCRIPTOR_SETS: &'static [&'static [u8]] = &[$($descriptor_set),+];
        }

        #[cfg(all(test, feature = "grpc"))]
        #[test]
        fn $test() {
            $crate::ext::assert_reflection_resolves::<$crate::$($server)::+<()>>($service);
        }
    };
}

#[cfg(all(test, feature = "grpc"))]
pub(crate) fn assert_reflection_resolves<S>(service: &str)
where
    S: ReflectionFiles + tonic::server::NamedService,
{
    assert_eq!(S::NAME, service);
    let mut pool = prost_reflect::DescriptorPool::new();
    pool.decode_file_descriptor_set(S::FILE_DESCRIPTOR_SETS.concat().as_slice())
        .unwrap();
    assert!(
        pool.get_service_by_name(service).is_some(),
        "{service} is missing from its own reflection files"
    );
}

pub(crate) use {impl_pair, impl_reflection_files, impl_validate};

#[cfg(test)]
mod tests {
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
        quote: String,
    }

    #[derive(Debug, Default)]
    struct Optionals {
        r#type: Option<String>,
        quote: Option<String>,
        count: u32,
        min: u32,
    }

    #[derive(Debug, Default)]
    struct Deep {
        outer: Option<Nested>,
        quote: Option<String>,
    }

    impl_pair! {
        Flat;
        paths = "base", "quote";
        infallible;
        base: [] base req;
        quote: [] quote req;
    }

    impl_pair! {
        Optionals;
        paths = "type", "quote";
        fallible;
        base: [] r#type opt;
        quote: [] quote opt;
    }

    impl_pair! {
        Deep;
        paths = "outer.leg.asset", "quote";
        fallible;
        base: [outer leg] asset req;
        quote: [] quote opt;
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
            quote: "USDC".into(),
        };
        assert_eq!(flat.pair(), ("SUI", "USDC"));
    }

    #[test]
    fn optional_leaves() {
        let full = Optionals {
            r#type: Some("SUI".into()),
            quote: Some("USDC".into()),
            ..Default::default()
        };
        assert_eq!(full.pair(), Some(("SUI", "USDC")));
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
            quote: Some("USDC".into()),
        };
        assert_eq!(deep.pair(), Some(("SUI", "USDC")));
        let no_leg = Deep {
            outer: Some(Nested::default()),
            quote: Some("USDC".into()),
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
