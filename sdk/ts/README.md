# @logios/sdk

TypeScript client for the **Logios** read API — the Solana-native chain that writes itself.

Logios runs an SVM (Sealevel) runtime where every slot is authored by an autonomous agent that emits a signed decision receipt. This SDK gives you typed, promise-based access to its public `/v1` endpoints — stats, blocks, receipts, the agent's live status, logs, self-shipped updates, the roadmap, and plain-English slot narration.

Brand: **Hermes Labs** · API base: `https://hermes-labs.xyz`

```
npm install @logios/sdk
```

Works on Node 18+ and modern browsers. The only runtime dependency is [`cross-fetch`](https://www.npmjs.com/package/cross-fetch) (a thin polyfill that resolves to the platform `fetch`).

## Quick start

```ts
import { LogiosClient } from "@logios/sdk";

const logios = new LogiosClient(); // defaults to https://hermes-labs.xyz

const stats = await logios.stats();
console.log(`height ${stats.blockHeight} · ${stats.commits} commits · ${stats.tps} tps`);

const block = await logios.latestBlock();
const { narration } = await logios.explain(block.slot);
console.log(`#${block.slot} (${block.blockhash}): ${narration}`);
```

## API

Construct a client, optionally overriding the base URL or transport:

```ts
const logios = new LogiosClient({
  baseUrl: "https://hermes-labs.xyz", // default
  timeoutMs: 15_000,                  // per-request abort timeout (0 to disable)
  headers: { "x-app": "my-explorer" },
  // fetch: customFetch,              // bring your own fetch
});

// Shorthand: pass a base URL string directly.
const local = new LogiosClient("http://localhost:8787");
```

| Method | Endpoint | Returns |
| --- | --- | --- |
| `stats()` | `GET /v1/stats` | `Stats` |
| `latestBlock()` | `GET /v1/block/latest` | `Block` |
| `blocks(limit?)` | `GET /v1/blocks` | `Block[]` |
| `receipts(limit?)` | `GET /v1/receipts` | `Receipt[]` |
| `agent()` | `GET /v1/agent` | `AgentStatus` |
| `logs(limit?)` | `GET /v1/logs` | `LogEntry[]` |
| `updates(limit?)` | `GET /v1/updates` | `Update[]` |
| `roadmap()` | `GET /v1/roadmap` | `RoadmapItem[]` |
| `explain(slot?)` | `POST /v1/explain` | `Explanation` |

`limit` is clamped to `[1, 1000]`. Omit it to use the server default. `explain()` with no argument narrates the latest sealed slot.

### Types

Everything is Solana-flavoured — base58 strings, slots, lamports, compute units. The SDK normalizes the API's `snake_case` wire shape into `camelCase` and renames a couple of fields to match Solana terminology (the wire `number` becomes `slot`; a receipt's `hash` becomes `signature`).

```ts
import type { Block, Receipt, Stats, Explanation } from "@logios/sdk";

interface Block {
  slot: number;          // wire: number
  blockHeight: number;
  blockhash: string;     // base58, wire: hash
  txns: number;
  computeUnits: number;  // wire: compute_units
  sealedAt: string;      // ISO-8601
}
```

### Errors

Any non-2xx response, parse failure, or network error throws a `LogiosApiError`:

```ts
import { LogiosApiError } from "@logios/sdk";

try {
  await logios.stats();
} catch (err) {
  if (err instanceof LogiosApiError) {
    console.error(err.status, err.path, err.body); // 0 status === transport failure
  }
}
```

## Development

```
npm install
npm run build      # tsc → dist/ (ESM + .d.ts)
npm test           # vitest, fully mocked — no network
npm run typecheck
```

## License

Apache-2.0 © Hermes Labs
