# Public API reference

The Hermes read API is served at `https://hermes-labs.xyz/v1`. It is read-only,
unauthenticated, and rate-limited per IP. All responses are JSON. Every hash,
pubkey, and signature is **base58**.

## Conventions

- **Base URL:** `https://hermes-labs.xyz/v1`
- **Encoding:** base58 for hashes / pubkeys / signatures; integers as JSON
  numbers; lamports and compute units are u64.
- **Errors:** non-2xx responses carry `{ "error": { "code", "message" } }`.

## Endpoints

### `GET /v1/health`

Node liveness and head state.

```json
{
  "ok": true,
  "slot": 12345,
  "leader": "Lead3r1111111111111111111111111111111111111",
  "jailed": false
}
```

### `GET /v1/slot/latest`

The most recently sealed slot. Same shape as `/v1/slot/{n}`.

### `GET /v1/slot/{n}`

A sealed slot by height.

```json
{
  "slot": 12345,
  "parent_blockhash": "9xQe...Hh2k",
  "compute_units": 18204113,
  "receipt_signature": "5Hd9...e3Qa"
}
```

Returns `404` if slot `n` is not yet sealed.

### `GET /v1/receipt/{sig}`

The decision receipt for a base58 signature.

```json
{
  "header": {
    "slot": 12345,
    "parent_blockhash": "9xQe...Hh2k",
    "leader": "Lead3r1111111111111111111111111111111111111"
  },
  "body": {
    "decision_summary": "ordered 42 tx by priority fee; no upgrade",
    "compute_units": 18204113,
    "account_delta_root": "7Gk2...Lm9p"
  },
  "signature": "5Hd9...e3Qa"
}
```

### `GET /v1/receipts?slot={n}&limit={k}`

Receipts walking backward from slot `n`, newest first. `limit` defaults to 20,
max 100.

```json
{
  "receipts": [ /* receipt objects */ ],
  "next_slot": 12325
}
```

### `GET /v1/leader`

Current leader pubkey and jail status.

```json
{
  "leader": "Lead3r1111111111111111111111111111111111111",
  "jailed": false,
  "jail_until_epoch": null
}
```

### `GET /v1/budget`

The per-slot compute budget and recent utilization.

```json
{
  "max_compute_units_per_slot": 48000000,
  "recent_avg_compute_units": 17650420,
  "samples": 64
}
```

## Notes

- The on-chain anchor for each receipt lives in the `receipt-registry` Anchor
  program; the API serves the same data from the node's ledger for convenience.
- Field names are stable within a `/v1` major; additive changes only.

<!-- maintained 2026-05-29 -->
