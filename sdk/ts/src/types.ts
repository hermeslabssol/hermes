/**
 * Type definitions for the Logios read API.
 *
 * Logios is a Solana-native chain (SVM / Sealevel runtime) where every slot is
 * authored by an autonomous agent. The vocabulary here is Solana's: base58
 * pubkeys/hashes/signatures, slots, lamports, compute units.
 *
 * @packageDocumentation
 */

/**
 * A base58-encoded string, as used throughout the SVM for blockhashes,
 * signatures and public keys. This is a nominal alias — it carries no runtime
 * guarantee, but documents intent at call sites and in returned shapes.
 *
 * @example "hz67m6UCc73QuCNtUwH8rwvt1gHbXR9Cg4d3TB2vWeky"
 */
export type Base58String = string;

/**
 * A base58-encoded ed25519 transaction/decision signature (64 bytes → ~88 chars).
 *
 * @example "NE7rwDLofnBsaEFm73dDE5Ju3s3oo43EGtSaufviHn8XFSQDgz3hviy62RqoguuBBB9ZYVo6CQwiqSJ7BHepDCrP"
 */
export type Signature = Base58String;

/**
 * A base58-encoded blockhash (32 bytes → ~44 chars).
 *
 * @example "9KdSJ4jDeG8YaWfAhazQTWgsQHX7UHZPtF6bkYS6krFM"
 */
export type Blockhash = Base58String;

/**
 * A slot number — the monotonic index of a sealed block on Logios.
 *
 * On the wire the API names this field `number`; the SDK surfaces it as `slot`
 * to match Solana terminology while keeping {@link Block.blockHeight} for the
 * raw value when you need it.
 */
export type Slot = number;

/**
 * An RFC-3339 / ISO-8601 timestamp string, as emitted by the API.
 *
 * @example "2026-06-06T16:20:00.01422+00:00"
 */
export type Timestamp = string;

/**
 * Network-wide counters and genesis info.
 *
 * Mirrors `GET /v1/stats`.
 */
export interface Stats {
  /** Height of the latest sealed slot. */
  blockHeight: number;
  /** Total number of decision receipts (signed commits) the agent has authored. */
  commits: number;
  /** Recent transactions-per-second throughput. */
  tps: number;
  /**
   * Validator set. Logios is single-leader: the autonomous agent validates its
   * own slots, so this is the literal string `"self"`.
   */
  validators: string;
  /** When the chain's genesis slot was sealed. */
  genesisAt: Timestamp;
}

/**
 * A sealed block (slot) on Logios.
 *
 * Mirrors entries from `GET /v1/block/latest` and `GET /v1/blocks`. The wire
 * shape uses `number`/`hash`; the SDK normalizes to Solana-flavoured names.
 */
export interface Block {
  /** The slot number. */
  slot: Slot;
  /** Raw block height (identical to {@link slot}; kept for explicitness). */
  blockHeight: number;
  /** base58 blockhash. */
  blockhash: Blockhash;
  /** Number of transactions packed into this slot. */
  txns: number;
  /** Compute units consumed by this slot (capped by the compute budget). */
  computeUnits: number;
  /** When the slot was sealed. */
  sealedAt: Timestamp;
}

/**
 * A signed decision receipt — the agent's record of why a slot was sealed.
 *
 * Mirrors entries from `GET /v1/receipts`.
 */
export interface Receipt {
  /** The slot this receipt attests to. */
  slot: Slot;
  /** Raw block number (identical to {@link slot}). */
  blockNumber: number;
  /** base58 ed25519 signature over the decision. */
  signature: Signature;
  /** Human-readable summary of what the agent decided for this slot. */
  decision: string;
  /** When the receipt was written to the ledger. */
  createdAt: Timestamp;
}

/**
 * Lifecycle phase the autonomous agent is currently in.
 *
 * The API returns an open-ended string (e.g. `"VERIFY"`, `"PACK"`, `"SEAL"`),
 * so this is a string with documented common values rather than a closed union.
 */
export type AgentPhase = string;

/**
 * Live status of the autonomous leader agent.
 *
 * Mirrors `GET /v1/agent`.
 */
export interface AgentStatus {
  /** Current lifecycle phase, e.g. `"VERIFY"`. */
  status: AgentPhase;
  /** What the agent is doing right now, e.g. `"packing transactions #4542502"`. */
  task: string;
  /** Agent build version, e.g. `"v0.4.1"`. */
  version: string;
  /** When this status was last refreshed. */
  updatedAt: Timestamp;
}

/** Severity / channel of a system log line. */
export type LogLevel = "ok" | "info" | "decision" | "warn" | "error" | (string & {});

/**
 * A single line from the agent's system log stream.
 *
 * Mirrors entries from `GET /v1/logs`.
 */
export interface LogEntry {
  /** Log severity / channel. */
  level: LogLevel;
  /** The log message. */
  message: string;
  /** When the line was emitted. */
  createdAt: Timestamp;
}

/**
 * A self-shipped protocol update — the chain rewriting itself.
 *
 * Mirrors entries from `GET /v1/updates`.
 */
export interface Update {
  /** Agent version that shipped the change, e.g. `"v0.4.18"`. */
  version: string;
  /** Short title of the change. */
  title: string;
  /** Longer description of what shipped. */
  body: string;
  /** When the update went live. */
  shippedAt: Timestamp;
}

/** Delivery state of a roadmap item. */
export type RoadmapStatus = "done" | "shipping" | "queued" | (string & {});

/**
 * A single roadmap entry, grouped into tiers and sections.
 *
 * Mirrors entries from `GET /v1/roadmap`.
 */
export interface RoadmapItem {
  /** Numeric tier (1 = Foundations, higher = further out). */
  tier: number;
  /** Human-readable tier name, e.g. `"Foundations"`. */
  tierName: string;
  /** Delivery status. */
  status: RoadmapStatus;
  /** Subsystem the item belongs to, e.g. `"chain"`, `"vm"`, `"api"`. */
  section: string;
  /** What the item is. */
  title: string;
}

/**
 * Plain-English narration of a single slot.
 *
 * Mirrors `POST /v1/explain`.
 */
export interface Explanation {
  /** Whether the explanation was produced successfully. */
  ok: boolean;
  /** The slot being narrated. */
  slot: Slot;
  /** Raw block number (identical to {@link slot}). */
  number: number;
  /** base58 blockhash of the slot. */
  blockhash: Blockhash;
  /** Transaction count for the slot. */
  txns: number;
  /** The human-readable narration of what happened in the slot. */
  narration: string;
}
