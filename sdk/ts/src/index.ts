/**
 * `@hermes/sdk` — TypeScript client for the Hermes read API.
 *
 * Hermes is the Solana-native chain that writes itself. This package gives you
 * typed access to its public `/v1` endpoints: stats, blocks, decision receipts,
 * the autonomous agent's status, logs, updates, the roadmap, and plain-English
 * slot narration.
 *
 * @example
 * ```ts
 * import { HermesClient } from "@hermes/sdk";
 *
 * const hermes = new HermesClient();
 * const stats = await hermes.stats();
 * console.log(`height ${stats.blockHeight} · ${stats.tps} tps`);
 * ```
 *
 * @packageDocumentation
 */

export { HermesClient, HermesApiError } from "./client.js";
export type { HermesClientOptions, ListOptions } from "./client.js";

export type {
  Base58String,
  Signature,
  Blockhash,
  Slot,
  Timestamp,
  Stats,
  Block,
  Receipt,
  AgentPhase,
  AgentStatus,
  LogLevel,
  LogEntry,
  Update,
  RoadmapStatus,
  RoadmapItem,
  Explanation,
} from "./types.js";

/** SDK version, kept in sync with package.json. */
export const VERSION = "0.5.0";
