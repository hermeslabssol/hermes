# Decision receipts

A decision receipt is the signed, deterministic record the agent emits for every
sealed slot. It is the artifact that makes an autonomous chain auditable. Full
spec: [RFC-0003](../spec/RFC-0003-decision-receipts.md).

## Shape

A receipt has a **header** and a **body**.

**Header** (cheap, light-client friendly):

| Field | Type | Notes |
| --- | --- | --- |
| `slot` | u64 | Slot height. |
| `parent_blockhash` | base58 hash | Blockhash of slot `n-1`. |
| `leader` | base58 pubkey | Authoring leader. |

**Body** (decision detail):

| Field | Type | Notes |
| --- | --- | --- |
| `decision_summary` | string | Transactions ordered, upgrade shipped. |
| `compute_units` | u64 | Total CU against the per-slot budget. |
| `account_delta_root` | base58 hash | SHA-256 commitment over sealed writes. |

## Canonical bytes and signature

Header and body serialize in a fixed field order with little-endian fixed-width
integers and length-prefixed variable fields, behind a leading
`schema_version` byte. Every node produces identical canonical bytes for the
same sealed slot.

The leader signs those bytes with Ed25519. The signature is stored and shown in
**base58**, never hex. To verify, recompute the canonical bytes from the sealed
slot and check the signature against `leader`.

## Where receipts live

- **Ledger** — `logios-ledger` appends one receipt per sealed slot, append-only,
  addressable by slot height and by base58 signature.
- **On-chain** — the `receipt-registry` Anchor program anchors each receipt's
  base58 signature on-chain, so a receipt's existence and ordering are verifiable
  from account state, not only a node's local log.

## Reading a receipt

```bash
curl https://hermes-labs.xyz/v1/receipt/<base58-signature>
curl "https://hermes-labs.xyz/v1/receipts?slot=12345&limit=20"
```

```bash
cargo run -p logios-cli -- receipt get <base58-signature>
```

## Why this matters

Because receipts are deterministic and signed, you do not have to trust a
description of what the agent intends. You read the signed record of what it did,
and a slashing proof for any equivocation is just a pair of conflicting receipts.
