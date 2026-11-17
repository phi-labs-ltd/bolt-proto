mod bolt_ext;
// bolt_ext.rs from a bolt-proto-extensions image older than the impl_*! calls doesn't
// use these macros. TODO: drop the allow once that image is published as `latest`.
#[allow(unused_macros, unused_imports)]
mod ext;
mod generated;
pub use generated::*;

#[cfg(feature = "num")]
mod bps;
mod numeric;
pub mod stream_cache;
