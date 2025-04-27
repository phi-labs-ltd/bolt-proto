# Oracle Service

The Oracle Service is a critical component of the Bolt Protocol that provides price feed data. This service enables the protocol to maintain accurate pricing information, which is essential for zero-slippage trading.

## Service Definition

The Oracle Service is defined in `bolt/outpost/v1/centralized_oracle.proto` and provides two main endpoints:

```protobuf
service OracleService {
  // Endpoint to post prices to the oracle
  rpc PostPrices(PostPricesRequest) returns (PostPricesResponse);
  // Endpoint to get the information about the network the oracle is running on
  rpc Info(InfoRequest) returns (InfoResponse);
}
```

## Message Types

### PostPricesRequest

This message is used to submit price updates to the oracle service.

```protobuf
message PostPricesRequest {
  // price_updates: List of price updates to be posted to the oracle
  repeated PriceUpdate price_updates = 1;
}
```

**Fields**:
- `price_updates` (repeated [PriceUpdate](#priceupdate)): A list of price updates to be posted to the oracle.

### PostPricesResponse

This message is the response to the `PostPricesRequest`. It does not contain any fields and is used simply to confirm that the request was processed.

```protobuf
message PostPricesResponse {}
```

### InfoRequest

This message is used to request information about the oracle service.

```protobuf
message InfoRequest {}
```

### InfoResponse

This message provides information about the oracle service, including which blockchain it's running on and the address of the signer.

```protobuf
message InfoResponse {
  // chain_id: The chain id of the chain the oracle is running on. e.g. archway-1
  string chain_id = 1;
  // whoami: The address of the signer who will be submitting the prices. e.g. archway1qwertyuiopasdfghjklzxcvbnm
  string whoami = 2;
}
```

**Fields**:
- `chain_id` (string): The identifier of the blockchain where the oracle is operating (e.g., "archway-1")
- `whoami` (string): The blockchain address of the signer that submits prices to the oracle

### PriceUpdate

This message represents a price update for a specific trading pair.

```protobuf
message PriceUpdate {
  // pair: The pair for which the price is being updated. e.g. ATOM/USDT
  Pair pair = 1;
  // price: The price of the pair. e.g. 10.0
  string price = 2;
  // timestamp: The timestamp of the price update. e.g. 2023-10-01T00:00:00Z
  google.protobuf.Timestamp timestamp = 3;
}
```

**Fields**:
- `pair` ([Pair](#pair)): The trading pair for which the price is being updated
- `price` (string): The price value as a string, allowing for decimal precision (e.g., "10.0")
- `timestamp` (google.protobuf.Timestamp): The timestamp when this price was recorded

### Pair

This message represents a trading pair consisting of a base asset and a quote asset.

```protobuf
message Pair {
  // base: The base asset of the pair. e.g. ATOM
  string base = 1;
  // quote: The quote asset of the pair. e.g. USDT
  string quote = 2;
}
```

**Fields**:
- `base` (string): The base asset of the pair (e.g., "ATOM")
- `quote` (string): The quote asset of the pair (e.g., "USDT")

## Usage Examples

### Posting Prices to the Oracle

The following example demonstrates how to post price updates to the oracle service:

#### Go Example
```go
import (
    "context"
    "time"
    
    pb "github.com/phi-labs-ltd/bolt-proto/bolt/outpost/v1"
    "google.golang.org/grpc"
    "google.golang.org/protobuf/types/known/timestamppb"
)

func postPrices(client pb.OracleServiceClient) error {
    // Create timestamp for now
    now := timestamppb.New(time.Now())
    
    // Create request with price updates
    req := &pb.PostPricesRequest{
        PriceUpdates: []*pb.PriceUpdate{
            {
                Pair: &pb.Pair{
                    Base:  "ATOM",
                    Quote: "USDT",
                },
                Price:     "12.34",
                Timestamp: now,
            },
            {
                Pair: &pb.Pair{
                    Base:  "ETH",
                    Quote: "USDT",
                },
                Price:     "2345.67",
                Timestamp: now,
            },
        },
    }
    
    // Send the request
    _, err := client.PostPrices(context.Background(), req)
    return err
}
```

#### Rust Example
```rust
use bolt::outpost::v1::{
    oracle_service_client::OracleServiceClient, Pair, PostPricesRequest, PriceUpdate,
};
use prost_types::Timestamp;
use tonic::Request;

async fn post_prices(client: &mut OracleServiceClient<tonic::transport::Channel>) -> Result<(), Box<dyn std::error::Error>> {
    // Create timestamp for now
    let now = Timestamp {
        seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64,
        nanos: 0,
    };
    
    // Create request with price updates
    let request = Request::new(PostPricesRequest {
        price_updates: vec![
            PriceUpdate {
                pair: Some(Pair {
                    base: "ATOM".to_string(),
                    quote: "USDT".to_string(),
                }),
                price: "12.34".to_string(),
                timestamp: Some(now.clone()),
            },
            PriceUpdate {
                pair: Some(Pair {
                    base: "ETH".to_string(),
                    quote: "USDT".to_string(),
                }),
                price: "2345.67".to_string(),
                timestamp: Some(now),
            },
        ],
    });
    
    // Send the request
    let _response = client.post_prices(request).await?;
    Ok(())
}
```

### Getting Oracle Information

The following example demonstrates how to get information about the oracle service:

#### Go Example
```go
import (
    "context"
    "fmt"
    
    pb "github.com/phi-labs-ltd/bolt-proto/bolt/outpost/v1"
    "google.golang.org/grpc"
)

func getOracleInfo(client pb.OracleServiceClient) error {
    // Create empty request
    req := &pb.InfoRequest{}
    
    // Send the request
    resp, err := client.Info(context.Background(), req)
    if err != nil {
        return err
    }
    
    // Use the response
    fmt.Printf("Oracle is running on chain: %s\n", resp.ChainId)
    fmt.Printf("Oracle signer address: %s\n", resp.Whoami)
    
    return nil
}
```

#### Rust Example
```rust
use bolt::outpost::v1::{oracle_service_client::OracleServiceClient, InfoRequest};
use tonic::Request;

async fn get_oracle_info(client: &mut OracleServiceClient<tonic::transport::Channel>) -> Result<(), Box<dyn std::error::Error>> {
    // Create empty request
    let request = Request::new(InfoRequest {});
    
    // Send the request
    let response = client.info(request).await?;
    let info = response.into_inner();
    
    // Use the response
    println!("Oracle is running on chain: {}", info.chain_id);
    println!("Oracle signer address: {}", info.whoami);
    
    Ok(())
}
```

## Role in Zero-Slippage Trading

The Oracle Service plays a crucial role in enabling zero-slippage trading by:

1. **Providing Accurate Price Feeds**: By maintaining up-to-date price information, the system can determine fair market prices for assets.

2. **Supporting On-Demand Liquidity**: Price information is essential for liquidity providers to correctly price their offers.

3. **Enabling Cross-Chain Operations**: By standardizing price information across different chains, the protocol can ensure consistent pricing in cross-chain transactions.

## Integration Considerations

When integrating with the Oracle Service, consider the following:

1. **Price Accuracy**: Ensure that price updates are accurate and timely to maintain the integrity of the system.

2. **Update Frequency**: Determine an appropriate frequency for price updates based on market volatility and system requirements.

3. **Error Handling**: Implement proper error handling to manage cases where the service is unavailable or returns errors.

4. **Authentication**: Ensure that only authorized entities can post prices to the oracle service. 