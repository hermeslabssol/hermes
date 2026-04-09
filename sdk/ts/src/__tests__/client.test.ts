import { afterEach, describe, expect, it, vi } from "vitest";

import { LogiosApiError, LogiosClient } from "../client.js";

/** Build a fake `fetch` that returns `body` as JSON for any URL. */
function mockFetch(body: unknown, init: { status?: number; statusText?: string } = {}) {
  const status = init.status ?? 200;
  return vi.fn(async (_url: string, _opts?: unknown) => {
    return {
      ok: status >= 200 && status < 300,
      status,
      statusText: init.statusText ?? "OK",
      text: async () => (typeof body === "string" ? body : JSON.stringify(body)),
    } as unknown as Response;
  });
}

function clientWith(fetchFn: ReturnType<typeof mockFetch>) {
  return new LogiosClient({ fetch: fetchFn as unknown as typeof fetch, timeoutMs: 0 });
}

afterEach(() => vi.restoreAllMocks());

describe("LogiosClient construction", () => {
  it("defaults to the hermes-labs origin and strips trailing slashes", () => {
    expect(new LogiosClient().baseUrl).toBe("https://hermes-labs.xyz");
    expect(new LogiosClient("https://example.test/").baseUrl).toBe("https://example.test");
    expect(new LogiosClient({ baseUrl: "https://x.test//" }).baseUrl).toBe("https://x.test");
  });
});

describe("stats()", () => {
  it("normalizes the first row of the stats array", async () => {
    const f = mockFetch([
      {
        block_height: 4542502,
        commits: 11496,
        tps: 12.1,
        validators: "self",
        genesis_at: "2026-02-21T12:44:27.606072+00:00",
      },
    ]);
    const stats = await clientWith(f).stats();
    expect(stats).toEqual({
      blockHeight: 4542502,
      commits: 11496,
      tps: 12.1,
      validators: "self",
      genesisAt: "2026-02-21T12:44:27.606072+00:00",
    });
    expect(f).toHaveBeenCalledWith(
      "https://hermes-labs.xyz/v1/stats",
      expect.objectContaining({ method: "GET" }),
    );
  });
});

describe("latestBlock()", () => {
  it("maps wire fields (number/hash) to Solana-flavoured names", async () => {
    const f = mockFetch([
      {
        number: 4542502,
        hash: "hz67m6UCc73QuCNtUwH8rwvt1gHbXR9Cg4d3TB2vWeky",
        txns: 130,
        compute_units: 48000000,
        sealed_at: "2026-06-06T16:20:00.01422+00:00",
      },
    ]);
    const block = await clientWith(f).latestBlock();
    expect(block.slot).toBe(4542502);
    expect(block.blockHeight).toBe(4542502);
    expect(block.blockhash).toBe("hz67m6UCc73QuCNtUwH8rwvt1gHbXR9Cg4d3TB2vWeky");
    expect(block.computeUnits).toBe(48000000);
    expect(block.sealedAt).toBe("2026-06-06T16:20:00.01422+00:00");
  });
});

describe("blocks(limit)", () => {
  it("clamps the limit and appends it as a query param", async () => {
    const f = mockFetch([]);
    await clientWith(f).blocks(5000);
    expect(f).toHaveBeenCalledWith(
      "https://hermes-labs.xyz/v1/blocks?limit=1000",
      expect.anything(),
    );
  });

  it("omits the query param when no limit is given", async () => {
    const f = mockFetch([]);
    await clientWith(f).blocks();
    expect(f).toHaveBeenCalledWith("https://hermes-labs.xyz/v1/blocks", expect.anything());
  });
});

describe("receipts()", () => {
  it("maps block_number → slot and hash → signature", async () => {
    const f = mockFetch([
      {
        block_number: 4542502,
        hash: "NE7rwDLofnBsaEFm73dDE5Ju3s3oo43EGtSaufviHn8XFSQDgz3hviy62RqoguuBBB9ZYVo6CQwiqSJ7BHepDCrP",
        decision: "sealed 130 txns; 130 account writes; program ran clean under the compute budget",
        created_at: "2026-06-06T16:20:00.01422+00:00",
      },
    ]);
    const [r] = await clientWith(f).receipts(1);
    expect(r?.slot).toBe(4542502);
    expect(r?.signature).toMatch(/^NE7rwD/);
    expect(r?.decision).toContain("sealed 130 txns");
  });
});

describe("explain()", () => {
  it("POSTs an empty body when no slot is given (latest)", async () => {
    const f = mockFetch({
      ok: true,
      number: 4542502,
      hash: "hz67m6UCc73QuCNtUwH8rwvt1gHbXR9Cg4d3TB2vWeky",
      txns: 130,
      narration: "Slot #4542502 sealed 130 transactions. Nobody approved it first.",
    });
    const ex = await clientWith(f).explain();
    expect(ex.slot).toBe(4542502);
    expect(ex.narration).toContain("Nobody approved it first");
    const body = (f.mock.calls[0]?.[1] as RequestInit)?.body;
    expect(body).toBe("{}");
  });

  it("POSTs { p_number } when a slot is given", async () => {
    const f = mockFetch({
      ok: true,
      number: 4542500,
      hash: "9KdSJ4jDeG8YaWfAhazQTWgsQHX7UHZPtF6bkYS6krFM",
      txns: 217,
      narration: "Slot #4542500 sealed 217 transactions.",
    });
    await clientWith(f).explain(4542500);
    const body = (f.mock.calls[0]?.[1] as RequestInit)?.body;
    expect(JSON.parse(body as string)).toEqual({ p_number: 4542500 });
  });
});

describe("error handling", () => {
  it("throws LogiosApiError on non-2xx with status + body", async () => {
    const f = mockFetch("upstream exploded", { status: 503, statusText: "Service Unavailable" });
    await expect(clientWith(f).stats()).rejects.toMatchObject({
      name: "LogiosApiError",
      status: 503,
      path: "/v1/stats",
      body: "upstream exploded",
    });
  });

  it("wraps transport failures with status 0", async () => {
    const f = vi.fn(async () => {
      throw new Error("ECONNREFUSED");
    });
    const err = await clientWith(f as unknown as ReturnType<typeof mockFetch>)
      .agent()
      .catch((e) => e);
    expect(err).toBeInstanceOf(LogiosApiError);
    expect(err.status).toBe(0);
    expect(err.cause).toBeInstanceOf(Error);
  });

  it("throws on unparseable JSON", async () => {
    const f = mockFetch("<html>not json</html>");
    await expect(clientWith(f).roadmap()).rejects.toBeInstanceOf(LogiosApiError);
  });
});
