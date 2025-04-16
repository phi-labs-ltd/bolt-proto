# bolt-proto

To build the proto files into generated types, use the examples folder.

## Rust

```sh
cd examples; buf generate --template buf.gen.rust.yaml
```

The proto files are created inside the `examples\rust` folder.

## Golang

```sh
cd examples; buf generate --template buf.gen.go.yaml
```

The proto files are created inside the `examples\go` folder.
