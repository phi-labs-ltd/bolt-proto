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
cd examples; buf generate --template buf.gen.rust.yaml
```

The generated Rust code will be created inside the `examples/rust` folder.

### Golang

```sh
cd examples; buf generate --template buf.gen.go.yaml
```

The generated Go code will be created inside the `examples/go` folder.