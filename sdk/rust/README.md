# hermes-client

Async Rust client for the **Hermes** read API — the Solana-native chain that writes itself.

Built on `reqwest` (async, rustls) + `serde`. Solana-flavoured throughout: base58 blockhashes/signatures, slots, compute units.

```toml
[dependencies]
hermes-client = "0.5"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Usage

```rust
use hermes_client::HermesClient;

#[tokio::main]
async fn main() -> Result<(), hermes_client::Error> {
    let hermes = HermesClient::new(); // https://hermes-labs.xyz

    let stats = hermes.stats().await?;
    println!("height {} · {} commits · {} tps", stats.block_height, stats.commits, stats.tps);

    let block = hermes.latest_block().await?;
    let ex = hermes.explain(Some(block.slot)).await?;
    println!("#{} ({}): {}", block.slot, block.blockhash, ex.narration);

    for r in hermes.receipts(Some(5)).await? {
        println!("  slot {} · {}", r.slot, r.decision);
    }
    Ok(())
}
```

## Methods

| Method | Endpoint |
| --- | --- |
| `stats()` | `GET /v1/stats` |
| `latest_block()` | `GET /v1/block/latest` |
| `blocks(limit)` | `GET /v1/blocks` |
| `receipts(limit)` | `GET /v1/receipts` |
| `agent()` | `GET /v1/agent` |
| `logs(limit)` | `GET /v1/logs` |
| `updates(limit)` | `GET /v1/updates` |
| `roadmap()` | `GET /v1/roadmap` |
| `explain(slot)` | `POST /v1/explain` |

`limit` is `Option<u32>`, clamped to `[1, 1000]`. `explain(None)` narrates the latest slot.

The client is cheap to `clone()` — the inner connection pool is shared. Point it elsewhere with `HermesClient::with_base_url("http://localhost:8787")`, or bring your own `reqwest::Client` via `with_client`.

## License

Apache-2.0 © Hermes Labs
