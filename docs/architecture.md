# Architecture

Hermes is a small, layered stack. State and verification flow upward from
primitives to the public surface. Control flows downward: each slot, the
autonomous agent drives the runtime, which feeds consensus, which feeds the
ledger.

## Layers

```
Agent (Hermes leader)
   │  slot plan
   ▼
hermes-runtime      SVM / Sealevel executor, compute-budget metering
   │  executed batch + CU total
   ▼
hermes-consensus    single-leader production, slashing + jail
   │  sealed slot
   ▼
hermes-ledger       append-only signed decision receipts
   │
   ├──► /v1 read API        slots · receipts · health
   └──► programs (Anchor)   receipt-registry · hermes-faucet
            │
            ├──► sdk/ts (@hermes/sdk)
            └──► sdk/rust (hermes-client) + cli
```

Everything is built on `hermes-primitives`, the dependency-light vocabulary
crate: base58 pubkeys, hashes, and Ed25519 signatures, plus the slot, epoch,
lamport, and compute-unit units and constants.

## Crates

- **`hermes-primitives`** — encoding (base58, never hex) and units. Defines
  `MAX_COMPUTE_UNITS_PER_SLOT = 48_000_000`, `LAMPORTS_PER_SOL`, and the 32-byte
  pubkey / 64-byte signature widths. `#![forbid(unsafe_code)]`.
- **`hermes-runtime`** — accounts owned by programs, transaction batches, and
  per-instruction compute-unit metering against the per-slot budget. See
  [`runtime.md`](runtime.md).
- **`hermes-consensus`** — the single autonomous leader, Tower-BFT vote tracking
  (WIP), and the slashing/jail engine. See [`consensus.md`](consensus.md).
- **`hermes-ledger`** — the append-only log of signed decision receipts, one per
  sealed slot. See [`receipts.md`](receipts.md).

## Programs (Anchor)

- **`receipt-registry`** — anchors each receipt's base58 signature on-chain so a
  receipt's existence is verifiable from account state.
- **`hermes-faucet`** — rate-limited devnet $LABS drip, one claim per pubkey
  per epoch.

## Clients

- **`@hermes/sdk`** (`sdk/ts`) — TypeScript client for `/v1`.
- **`hermes-client`** (`sdk/rust`) — Rust client over `/v1` and the programs.
- **`hermes-cli`** (`cli`) — operator and explorer CLI.

## Determinism

The runtime is deterministic: same starting accounts plus same ordered batch
yield the same deltas and the same total compute units on every node. That is
what lets the decision receipt's signature be verified independently, and what
lets a slashing proof be reconstructed from receipts alone.

## Data flow per slot

See [`RFC-0001`](../spec/RFC-0001-autonomous-block-authoring.md) for the full
loop. In short: propose → write accounts → build → execute under budget → seal
→ sign receipt.
