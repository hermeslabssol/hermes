/**
 * {@link LogiosClient} — a typed client for the Logios read API.
 *
 * @packageDocumentation
 */

import fetch from "cross-fetch";

import type {
  AgentStatus,
  Block,
  Explanation,
  LogEntry,
  Receipt,
  RoadmapItem,
  Slot,
  Stats,
  Update,
} from "./types.js";

/**
 * Error thrown when the Logios API returns a non-2xx response, an unparseable
 * body, or a network failure occurs.
 *
 * Inspect {@link status} / {@link body} for the HTTP detail, or
 * {@link cause} for an underlying transport error.
 */
export class LogiosApiError extends Error {
  /** HTTP status code, or `0` for transport-level failures. */
  public readonly status: number;
  /** Request path that failed, e.g. `"/v1/stats"`. */
  public readonly path: string;
  /** Raw response body, when one was read. */
  public readonly body?: string;
  /** Underlying error (network, parse), when applicable. */
  public readonly cause?: unknown;

  constructor(
    message: string,
    opts: { status: number; path: string; body?: string; cause?: unknown },
  ) {
    super(message);
    this.name = "LogiosApiError";
    this.status = opts.status;
    this.path = opts.path;
    this.body = opts.body;
    this.cause = opts.cause;
    // Restore prototype chain for instanceof across transpile targets.
    Object.setPrototypeOf(this, LogiosApiError.prototype);
  }
}

/** Options accepted by the {@link LogiosClient} constructor. */
export interface LogiosClientOptions {
  /**
   * Base origin of the API, without the `/v1` suffix.
   * @defaultValue `"https://hermes-labs.xyz"`
   */
  baseUrl?: string;
  /**
   * Per-request timeout in milliseconds. Set to `0` to disable.
   * @defaultValue `15000`
   */
  timeoutMs?: number;
  /**
   * Custom fetch implementation. Defaults to {@link cross-fetch}, which resolves
   * to the platform `fetch` on modern Node / browsers.
   */
  fetch?: typeof fetch;
  /** Extra headers merged into every request. */
  headers?: Record<string, string>;
}

/** Common shape for list endpoints that accept a row limit. */
export interface ListOptions {
  /**
   * Maximum number of rows to return. Omit to use the server default.
   * Clamped to `[1, 1000]` when provided.
   */
  limit?: number;
}

const DEFAULT_BASE_URL = "https://hermes-labs.xyz";
const DEFAULT_TIMEOUT_MS = 15_000;

/**
 * Typed client for the Logios public read API (`/v1`).
 *
 * Every method maps to one endpoint, returning the normalized SDK types from
 * {@link ./types}. The underlying API is backed by PostgREST, so list endpoints
 * return arrays and accept a `limit` query parameter.
 *
 * @example
 * ```ts
 * import { LogiosClient } from "@logios/sdk";
 *
 * const logios = new LogiosClient();
 * const { slot } = await logios.latestBlock();
 * const { narration } = await logios.explain(slot);
 * console.log(`#${slot}: ${narration}`);
 * ```
 */
export class LogiosClient {
  /** Resolved base origin, no trailing slash. */
  public readonly baseUrl: string;
  private readonly timeoutMs: number;
  private readonly fetchImpl: typeof fetch;
  private readonly headers: Record<string, string>;

  /**
   * @param options Configuration, or a bare base-URL string for convenience.
   */
  constructor(options: LogiosClientOptions | string = {}) {
    const opts: LogiosClientOptions =
      typeof options === "string" ? { baseUrl: options } : options;

    this.baseUrl = (opts.baseUrl ?? DEFAULT_BASE_URL).replace(/\/+$/, "");
    this.timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    this.fetchImpl = opts.fetch ?? fetch;
    this.headers = {
      accept: "application/json",
      "user-agent": "logios-sdk-ts/0.5.0",
      ...opts.headers,
    };
  }

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /**
   * Fetch network-wide counters and genesis info.
   * Maps to `GET /v1/stats`.
   */
  async stats(): Promise<Stats> {
    const [row] = await this.getList<StatsWire>("/v1/stats");
    if (!row) throw new LogiosApiError("stats endpoint returned no rows", { status: 200, path: "/v1/stats" });
    return {
      blockHeight: row.block_height,
      commits: row.commits,
      tps: row.tps,
      validators: row.validators,
      genesisAt: row.genesis_at,
    };
  }

  /**
   * Fetch the most recently sealed block.
   * Maps to `GET /v1/block/latest`.
   */
  async latestBlock(): Promise<Block> {
    const [row] = await this.getList<BlockWire>("/v1/block/latest");
    if (!row) throw new LogiosApiError("no latest block available", { status: 200, path: "/v1/block/latest" });
    return mapBlock(row);
  }

  /**
   * Fetch a window of recent blocks, newest first.
   * Maps to `GET /v1/blocks`.
   *
   * @param limit Max rows (1–1000).
   */
  async blocks(limit?: number): Promise<Block[]> {
    const rows = await this.getList<BlockWire>("/v1/blocks", limit);
    return rows.map(mapBlock);
  }

  /**
   * Fetch recent decision receipts, newest first.
   * Maps to `GET /v1/receipts`.
   *
   * @param limit Max rows (1–1000).
   */
  async receipts(limit?: number): Promise<Receipt[]> {
    const rows = await this.getList<ReceiptWire>("/v1/receipts", limit);
    return rows.map((r) => ({
      slot: r.block_number,
      blockNumber: r.block_number,
      signature: r.hash,
      decision: r.decision,
      createdAt: r.created_at,
    }));
  }

  /**
   * Fetch the autonomous agent's current status.
   * Maps to `GET /v1/agent`.
   */
  async agent(): Promise<AgentStatus> {
    const [row] = await this.getList<AgentWire>("/v1/agent");
    if (!row) throw new LogiosApiError("agent endpoint returned no rows", { status: 200, path: "/v1/agent" });
    return {
      status: row.status,
      task: row.task,
      version: row.version,
      updatedAt: row.updated_at,
    };
  }

  /**
   * Fetch recent lines from the agent's system log stream, newest first.
   * Maps to `GET /v1/logs`.
   *
   * @param limit Max rows (1–1000).
   */
  async logs(limit?: number): Promise<LogEntry[]> {
    const rows = await this.getList<LogWire>("/v1/logs", limit);
    return rows.map((l) => ({
      level: l.level,
      message: l.message,
      createdAt: l.created_at,
    }));
  }

  /**
   * Fetch recent self-shipped protocol updates, newest first.
   * Maps to `GET /v1/updates`.
   *
   * @param limit Max rows (1–1000).
   */
  async updates(limit?: number): Promise<Update[]> {
    const rows = await this.getList<UpdateWire>("/v1/updates", limit);
    return rows.map((u) => ({
      version: u.version,
      title: u.title,
      body: u.body,
      shippedAt: u.shipped_at,
    }));
  }

  /**
   * Fetch the full roadmap, ordered by tier.
   * Maps to `GET /v1/roadmap`.
   */
  async roadmap(): Promise<RoadmapItem[]> {
    const rows = await this.getList<RoadmapWire>("/v1/roadmap");
    return rows.map((r) => ({
      tier: r.tier,
      tierName: r.tier_name,
      status: r.status,
      section: r.section,
      title: r.title,
    }));
  }

  /**
   * Get a plain-English narration of a slot.
   * Maps to `POST /v1/explain`.
   *
   * @param slot The slot to explain. Omit to narrate the latest sealed slot.
   */
  async explain(slot?: Slot): Promise<Explanation> {
    const path = "/v1/explain";
    // The RPC takes a `p_number` argument; an empty body narrates the latest slot.
    const payload: Record<string, unknown> =
      slot === undefined ? {} : { p_number: slot };

    const raw = await this.request<ExplanationWire>(path, {
      method: "POST",
      body: JSON.stringify(payload),
      headers: { "content-type": "application/json" },
    });

    return {
      ok: raw.ok,
      slot: raw.number,
      number: raw.number,
      blockhash: raw.hash,
      txns: raw.txns,
      narration: raw.narration,
    };
  }

  // ---------------------------------------------------------------------------
  // Internals
  // ---------------------------------------------------------------------------

  /** GET a PostgREST-style list endpoint, optionally with a `limit`. */
  private async getList<T>(path: string, limit?: number): Promise<T[]> {
    let url = path;
    if (limit !== undefined) {
      const clamped = Math.max(1, Math.min(1000, Math.trunc(limit)));
      url += `?limit=${clamped}`;
    }
    const data = await this.request<T[] | T>(url, { method: "GET" });
    // Single-object endpoints (e.g. /v1/block/latest under some configs) are
    // tolerated by wrapping into an array.
    return Array.isArray(data) ? data : [data];
  }

  /** Perform a request, parse JSON, and translate failures into {@link LogiosApiError}. */
  private async request<T>(path: string, init: RequestInit): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const controller =
      this.timeoutMs > 0 && typeof AbortController !== "undefined"
        ? new AbortController()
        : undefined;
    const timer =
      controller !== undefined
        ? setTimeout(() => controller.abort(), this.timeoutMs)
        : undefined;

    let res: Response;
    try {
      res = await this.fetchImpl(url, {
        ...init,
        headers: { ...this.headers, ...(init.headers as Record<string, string>) },
        signal: controller?.signal,
      });
    } catch (cause) {
      throw new LogiosApiError(`request to ${path} failed: ${stringifyError(cause)}`, {
        status: 0,
        path,
        cause,
      });
    } finally {
      if (timer !== undefined) clearTimeout(timer);
    }

    const text = await res.text().catch(() => "");

    if (!res.ok) {
      throw new LogiosApiError(`${res.status} ${res.statusText} for ${path}`, {
        status: res.status,
        path,
        body: text,
      });
    }

    if (text.length === 0) {
      return undefined as unknown as T;
    }

    try {
      return JSON.parse(text) as T;
    } catch (cause) {
      throw new LogiosApiError(`failed to parse JSON from ${path}`, {
        status: res.status,
        path,
        body: text,
        cause,
      });
    }
  }
}

function mapBlock(b: BlockWire): Block {
  return {
    slot: b.number,
    blockHeight: b.number,
    blockhash: b.hash,
    txns: b.txns,
    computeUnits: b.compute_units,
    sealedAt: b.sealed_at,
  };
}

function stringifyError(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}

// -----------------------------------------------------------------------------
// Wire shapes — the raw snake_case JSON returned by the API. Kept private so
// consumers only ever see the normalized types from ./types.
// -----------------------------------------------------------------------------

interface StatsWire {
  block_height: number;
  commits: number;
  tps: number;
  validators: string;
  genesis_at: string;
}

interface BlockWire {
  number: number;
  hash: string;
  txns: number;
  compute_units: number;
  sealed_at: string;
}

interface ReceiptWire {
  block_number: number;
  hash: string;
  decision: string;
  created_at: string;
}

interface AgentWire {
  status: string;
  task: string;
  version: string;
  updated_at: string;
}

interface LogWire {
  level: string;
  message: string;
  created_at: string;
}

interface UpdateWire {
  version: string;
  title: string;
  body: string;
  shipped_at: string;
}

interface RoadmapWire {
  tier: number;
  tier_name: string;
  status: string;
  section: string;
  title: string;
}

interface ExplanationWire {
  ok: boolean;
  number: number;
  hash: string;
  txns: number;
  narration: string;
}

// updated 2026-05-25
