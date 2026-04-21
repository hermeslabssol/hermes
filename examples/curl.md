# Logios `/v1` — raw curl examples

The Logios read API lives at `https://hermes-labs.xyz/v1`. It is backed by
PostgREST, so list endpoints return JSON arrays and accept a `limit` query
parameter. Everything is Solana-flavoured: base58 blockhashes/signatures, slots,
compute units.

All GET endpoints are public and require no auth.

## `GET /v1/stats`

Network-wide counters and genesis info.

```sh
curl -s https://hermes-labs.xyz/v1/stats
```

```json
[{"block_height":4542502,"commits":11496,"tps":12.1,"validators":"self","genesis_at":"2026-02-21T12:44:27.606072+00:00"}]
```

## `GET /v1/block/latest`

The most recently sealed block.

```sh
curl -s https://hermes-labs.xyz/v1/block/latest
```

```json
[{"number":4542502,"hash":"hz67m6UCc73QuCNtUwH8rwvt1gHbXR9Cg4d3TB2vWeky","txns":130,"compute_units":48000000,"sealed_at":"2026-06-06T16:20:00.01422+00:00"}]
```

## `GET /v1/blocks`

A window of recent blocks, newest first. Use `?limit=N`.

```sh
curl -s "https://hermes-labs.xyz/v1/blocks?limit=5"
```

## `GET /v1/receipts`

Signed decision receipts — the agent's record of why each slot was sealed. The
`hash` field is a base58 ed25519 signature.

```sh
curl -s "https://hermes-labs.xyz/v1/receipts?limit=5"
```

```json
[{"block_number":4542502,"hash":"NE7rwDLofnBsaEFm73dDE5Ju3s3oo43EGtSaufviHn8XFSQDgz3hviy62RqoguuBBB9ZYVo6CQwiqSJ7BHepDCrP","decision":"sealed 130 txns; 130 account writes; program ran clean under the compute budget","created_at":"2026-06-06T16:20:00.01422+00:00"}]
```

## `GET /v1/agent`

Live status of the autonomous leader agent.

```sh
curl -s https://hermes-labs.xyz/v1/agent
```

```json
[{"status":"VERIFY","task":"packing transactions #4542502","version":"v0.4.1","updated_at":"2026-06-06T16:20:00.01422+00:00"}]
```

## `GET /v1/logs`

Recent lines from the agent's system log stream. Use `?limit=N`.

```sh
curl -s "https://hermes-labs.xyz/v1/logs?limit=10"
```

```json
[{"level":"ok","message":"slot #4542502 sealed","created_at":"2026-06-06T16:20:00.01422+00:00"}]
```

## `GET /v1/updates`

Self-shipped protocol updates — the chain rewriting itself. Use `?limit=N`.

```sh
curl -s "https://hermes-labs.xyz/v1/updates?limit=5"
```

```json
[{"version":"v0.4.18","title":"Re-derived priority-fee curve","body":"Shipped without review. Program ran clean, accounts migrated in place.","shipped_at":"2026-06-06T16:14:00.064003+00:00"}]
```

## `GET /v1/roadmap`

The full roadmap, grouped into tiers and sections.

```sh
curl -s https://hermes-labs.xyz/v1/roadmap
```

```json
[{"tier":1,"tier_name":"Foundations","status":"done","section":"chain","title":"Slot-sealing loop with deterministic ordering"}]
```

## `POST /v1/explain`

Plain-English narration of a slot. Send `{ "p_number": <slot> }` to narrate a
specific slot, or an empty body `{}` to narrate the latest sealed slot.

```sh
# narrate a specific slot
curl -s -X POST https://hermes-labs.xyz/v1/explain \
  -H "content-type: application/json" \
  -d '{"p_number":4542500}'

# narrate the latest slot
curl -s -X POST https://hermes-labs.xyz/v1/explain \
  -H "content-type: application/json" \
  -d '{}'
```

```json
{"ok":true,"number":4542500,"hash":"9KdSJ4jDeG8YaWfAhazQTWgsQHX7UHZPtF6bkYS6krFM","txns":217,"narration":"Slot #4542500 sealed 217 transactions. The agent applied 217 account writes, ran the program clean under the compute budget, and the slot was signed. Nobody approved it first."}
```
