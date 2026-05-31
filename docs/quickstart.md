# Quickstart

Get Hermes building locally and read live slots from the public API.

## Prerequisites

- **Rust 1.79+** (pinned in [`rust-toolchain.toml`](../rust-toolchain.toml))
- **Solana toolchain** for `cargo build-sbf`
- **Anchor 0.30+** for the on-chain programs
- **Node 20+** for the TypeScript SDK

## Build the workspace

```bash
git clone https://github.com/hermeslabssol/hermes
cd hermes

cargo build          # primitives, runtime, consensus, ledger, sdk/rust, cli
cargo test --all     # core test suite
```

## Build the Anchor programs

```bash
anchor build         # receipt-registry, hermes-faucet
```

CI runs `anchor build` in a continue-on-error step, so a missing local Anchor
toolchain will not block the rest of the build.

## Run the TypeScript SDK example

```bash
cd sdk/ts
npm ci
npm run build
node examples/stream-slots.js
```

## Read the public API

The read API is live at `https://hermes-labs.xyz/v1`. No auth required.

```bash
curl https://hermes-labs.xyz/v1/health
curl https://hermes-labs.xyz/v1/slot/latest
curl https://hermes-labs.xyz/v1/receipt/<base58-signature>
```

## Use the CLI

```bash
cargo run -p hermes-cli -- slot latest
cargo run -p hermes-cli -- receipt get <base58-signature>
cargo run -p hermes-cli -- leader status
```

## Next steps

- Read the [architecture overview](architecture.md).
- Understand [decision receipts](receipts.md) — the core auditable artifact.
- Browse the [API reference](api.md) and the [RFCs](../spec/).
