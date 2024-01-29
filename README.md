# ETH RPC Cache Layer
A simple http server to cache specific eth rpc requests in memory. Useful for massive repeatedly requests to ETH rpc endpoints. 
Multiple endpoints/chains can be configured to be cached.

### Usage (with docker)

1. Edit `docker-compose.yml`

2. `docker-compose up`
### Usage
With
```shell
cargo run --release -- \
  --port=8124 \
  --bind=0.0.0.0 \
  --endpoint=eth=https://rpc.ankr.com/eth \
  --endpoint=bsc=https://rpc.ankr.com/bsc \
  --redis-url=redis://localhost:6379
```
Following redirection will be made:
* http://localhost:8124/eth -> https://rpc.ankr.com/eth
* http://localhost:8124/bsc -> https://rpc.ankr.com/bsc

### Supported methods
Mainly supported requests with determined block number. Other methods will be directly send to the configured ETH rpc endpoint.

- `eth_call`
- `eth_chainId`
- `eth_getBalance`
- `eth_getBlockByNumber`
- `eth_getBlockReceipts`
- `eth_getCode`
- `eth_getStorageAt`
- `eth_getTransactionByHash`
- `eth_getTransactionCount`
- `eth_getTransactionReceipt`
- `eth_getTransactionByBlockNumberAndIndex`
- `eth_getTransactionByBlockHashAndIndex`
