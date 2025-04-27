# Bolt Protocol Documentation

## Protocol Overview

Bolt Protocol is designed to enable efficient, zero-slippage trading and cross-chain operations through a well-defined set of message types. This protocol supports Bolt's core features:

- **Zero Slippage Trading**: Enables trading without price impact
- **On-Demand Liquidity**: Facilitates immediate access to liquidity when needed
- **Cross-Chain Compatibility**: Allows seamless operations across different blockchain networks

### Purpose of This Documentation

This documentation explains the Protocol Buffer definitions that form the foundation of Bolt's communication layer. It serves as a reference for developers integrating with Bolt's systems, helping to:

1. Understand the message structures used throughout the protocol
2. Implement clients that can communicate with Bolt services
3. Interpret the data flowing through the system

## Repository Structure

The repository is organized as follows:

```
bolt-proto/
├── bolt/                 # Main protocol definitions
│   └── outpost/          # Outpost-related protocol definitions
│       └── v1/           # Version 1 of the outpost protocol
│           └── centralized_oracle.proto  # Oracle service definitions
├── examples/             # Examples and code generation templates
│   ├── buf.gen.rust.yaml # Configuration for Rust code generation
│   └── buf.gen.go.yaml   # Configuration for Go code generation
└── docs/                 # Documentation (you are here)
```

## Getting Started

To use these protocol definitions in your project, you'll need to:

1. Clone this repository
2. Generate code for your target language
3. Import the generated code in your application

### Code Generation

#### For Rust

```sh
cd examples
buf generate --template buf.gen.rust.yaml
```

The generated Rust code will be available in the `examples/rust` directory.

#### For Go

```sh
cd examples
buf generate --template buf.gen.go.yaml
```

The generated Go code will be available in the `examples/go` directory.

## Next Steps

Explore the detailed documentation for each service:

- [Oracle Service](./oracle_service.md) - For price feed integration 