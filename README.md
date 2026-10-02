# bolt-proto

This repository contains the Protocol Buffer definitions for Bolt Protocol, which enables zero-slippage trading, on-demand liquidity, and cross-chain compatibility.

## Documentation

Comprehensive documentation is available in the `docs` directory:

- [Protocol Overview](./docs/protocol_overview.md) - High-level overview of the protocol
- [Oracle Service](./docs/oracle_service.md) - Detailed documentation for the Oracle Service
- [Message Flow](./docs/message_flow.md) - Visualization of how messages flow through the system

## Code Generation

To build the proto files into generated types, use the examples folder.

### Rust

```sh
just rust
```

This produces two crates in the `rust/` workspace, ready to publish to kellnr:

- `rust/bolt` is `bolt-proto`, generated from `bolt/`.
- `rust/cex` is `cex-proto`, generated from `cex/`. It depends on `bolt-proto` for the shared `bolt.*` types.

For each crate:

- `Cargo.toml` is committed and holds the crate version — bump it there before publishing.
- `src/` is generated and gitignored: one module file per proto package, plus a `lib.rs`
  exposing the module tree (`bolt::{domain}::{subdomain}::v{N}` or `cex::{subdomain}::v{N}`).

A release publishes both crates at the version in the release title.

Publishing requires the kellnr index URL in `.cargo/config.toml` and a token in
`CARGO_REGISTRIES_KELLNR_TOKEN`:

```sh
cd rust && cargo publish --package bolt-proto --registry kellnr
cd rust && cargo publish --package cex-proto --registry kellnr
```

### Golang

```sh
buf generate --template buf.gen.go.yaml
```

The generated Go code will be created inside the `go` folder.
