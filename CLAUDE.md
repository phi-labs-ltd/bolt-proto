# bolt-proto — AI Working Guide

## Tooling

- **Buf** enforces lint (`STANDARD`, `COMMENTS`, `FILE_LOWER_SNAKE_CASE`) and breaking-change detection (`FILE`)
- Run `buf format -w` then `buf lint` before committing; lint violations are errors

---

## Rules

### Files

- File names: `lower_snake_case.proto`
- One file per service; place at `bolt/{domain}/{subdomain}/v{N}/{name}.proto`
- Package must match the directory path exactly: `bolt/outpost/market_ledger/v1/` → `package bolt.outpost.market_ledger.v1;`
- Enums live in `types.proto` in the same directory as the service file; import via `"bolt/{domain}/{subdomain}/v{N}/types.proto"`
- Asset identifiers are `string` (e.g. `"SUI"`, `"USDC"`) — never a hardcoded enum; new assets must not require a proto release

### Enums

- Enum values must be prefixed with the enum type name in `UPPER_SNAKE_CASE`
- Zero value must be `{PREFIX}_UNSPECIFIED = 0`
- Example:
  ```protobuf
  enum OrderStatus {
    ORDER_STATUS_UNSPECIFIED = 0;
    ORDER_STATUS_PENDING     = 1;
    ORDER_STATUS_FILLED      = 2;
  }
  ```
- `-Type` suffix: for classification enums (`SwapType`, `OrderType`, `CexOrderType`)
- `-Status` suffix: for lifecycle enums (`LedgerStatus`, `CexOrderStatus`, `OrderStatus`)

### Messages

- PascalCase names; no nested messages
- Every RPC has an explicit top-level `{Rpc}Request` / `{Rpc}Response` pair
- Empty messages still defined explicitly: `message GetFooRequest {}`

### Fields

- Names: `snake_case`
- Decimal/financial values: `bolt.common.numeric.v1.Fraction` — never plain `string`; avoids cross-system decimal ambiguity
- Block heights: `uint64`
- DB primary keys: `int64 id`; foreign keys: `int64 {table}_id`
- External system IDs (CEX order IDs, tx digests): `string`
- Timestamps: `google.protobuf.Timestamp`
- Optional fields: use `optional` keyword

### Services & RPCs

- Service names: PascalCase ending in `Service`
- RPC names: PascalCase action verb — `GetFoo`, `UpsertFoo`, `DeleteFoo`
- Streaming RPCs: prefix with `Stream` or `Subscribe` — `SubscribeMarketLedger`, `StreamPrices`

### Comments (enforced by `COMMENTS` lint rule)

Every service, RPC, message, enum, enum value, and field **must** have a comment or buf lint will fail.

- Messages: `// {MessageName}: {description}.`
- Fields: `// {field_name}: {description}.`
- RPCs: `// Endpoint to {action}.` or `// {description}.`
- Enum values: `// {VALUE}: {description}.`
- Include units and examples where useful: `// price: Execution price as a decimal string (e.g. "1.25").`

### Imports

- Use fully qualified paths: `"bolt/assets/v1/assets.proto"`

### Breaking Changes

- `buf breaking --against` is enforced on CI against the published BSR module
- Never remove fields, messages, or enum values — mark reserved instead:
  ```protobuf
  reserved 3;
  reserved "old_field_name";
  ```
- Never change a field number or type
- Adding optional fields and new enum values is safe
