/**
 * `@logios/sdk` — TypeScript client for the Logios read API.
 *
 * Logios is the Solana-native chain that writes itself. This package gives you
 * typed access to its public `/v1` endpoints: stats, blocks, decision receipts,
 * the autonomous agent's status, logs, updates, the roadmap, and plain-English
 * slot narration.
 *
 * @example
 * ```ts
 * import { LogiosClient } from "@logios/sdk";
 *
 * const logios = new LogiosClient();
 * const stats = await logios.stats();
 * console.log(`height ${stats.blockHeight} · ${stats.tps} tps`);
 * ```
 *
 * @packageDocumentation
 */

export { LogiosClient, LogiosApiError } from "./client.js";
export type { LogiosClientOptions, ListOptions } from "./client.js";

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
