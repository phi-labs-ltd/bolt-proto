//! Generated Rust types and gRPC clients/servers for the Bolt Protocol protobuf schemas,
//! plus hand-written helpers such as [`stream_cache`].
//!
//! The generated module tree lives in `generated.rs` (written by `buf generate`); everything
//! else under `src/` that is not named `bolt.*.rs` is hand-written and committed.

mod generated;
pub use generated::*;

pub mod stream_cache;
