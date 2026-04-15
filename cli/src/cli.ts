#!/usr/bin/env node
/**
 * `logios` — command-line explorer for the Logios chain.
 *
 * A thin, friendly wrapper over `@logios/sdk`. Read-only against the public
 * `/v1` API, except `faucet`, which requests devnet SOL from the Solana RPC.
 */

import { LogiosApiError, LogiosClient, type Block } from "@logios/sdk";

import { c, colorLevel } from "./colors.js";

const PKG_VERSION = "0.5.0";
const DEFAULT_RPC = "https://api.devnet.solana.com";

interface GlobalFlags {
  baseUrl: string;
  json: boolean;
}

main().catch((err) => {
  fail(err);
});

async function main(): Promise<void> {
  const argv = process.argv.slice(2);

  if (argv.length === 0 || hasFlag(argv, "--help", "-h")) {
    printHelp();
    return;
  }
  if (hasFlag(argv, "--version", "-V")) {
    console.log(PKG_VERSION);
    return;
  }

  const baseUrl = takeOption(argv, "--url") ?? "https://hermes-labs.xyz";
  const json = hasFlag(argv, "--json");
  const flags: GlobalFlags = { baseUrl, json };

  const [command, ...rest] = argv.filter((a) => !a.startsWith("-"));
  const client = new LogiosClient({ baseUrl });

  switch (command) {
    case "status":
      return cmdStatus(client, flags);
    case "slot":
      return cmdSlot(client, flags, rest[0]);
    case "receipts":
      return cmdReceipts(client, flags, rest[0]);
    case "explain":
      return cmdExplain(client, flags, rest[0]);
    case "watch":
      return cmdWatch(client, flags);
    case "faucet":
      return cmdFaucet(flags, rest[0]);
    default:
      console.error(c.red(`unknown command: ${command ?? "(none)"}\n`));
      printHelp();
      process.exitCode = 1;
  }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/** `logios status` — network + agent overview. */
async function cmdStatus(client: LogiosClient, flags: GlobalFlags): Promise<void> {
  const [stats, agent, block] = await Promise.all([
    client.stats(),
    client.agent(),
    client.latestBlock(),
  ]);

  if (flags.json) return printJson({ stats, agent, latestBlock: block });

  header("Logios — the chain that writes itself");
  row("slot height", c.bold(String(stats.blockHeight)));
  row("commits", String(stats.commits));
  row("tps", String(stats.tps));
  row("validators", stats.validators);
  row("genesis", c.dim(stats.genesisAt));
  console.log();
  row("agent", `${colorLevel(agent.status)} ${c.dim(agent.version)}`);
  row("task", agent.task);
  console.log();
  row("latest slot", c.cyan(String(block.slot)));
  row("blockhash", c.gray(block.blockhash));
  row("txns / CU", `${block.txns} / ${block.computeUnits.toLocaleString("en-US")}`);
}

/** `logios slot [n]` — one block (latest if `n` omitted) + its narration. */
async function cmdSlot(client: LogiosClient, flags: GlobalFlags, arg?: string): Promise<void> {
  let block: Block;
  if (arg === undefined) {
    block = await client.latestBlock();
  } else {
    const n = parseSlot(arg);
    const blocks = await client.blocks(1000);
    const found = blocks.find((b) => b.slot === n);
    if (!found) {
      throw new Error(`slot ${n} not found in the recent window (try a value near the head)`);
    }
    block = found;
  }

  const ex = await client.explain(block.slot).catch(() => undefined);

  if (flags.json) return printJson({ block, explanation: ex });

  header(`Slot #${block.slot}`);
  row("blockhash", c.gray(block.blockhash));
  row("txns", String(block.txns));
  row("compute units", block.computeUnits.toLocaleString("en-US"));
  row("sealed at", c.dim(block.sealedAt));
  if (ex) {
    console.log();
    console.log(c.italic(wrapText(ex.narration, 76)));
  }
}

/** `logios receipts [limit]` — recent signed decision receipts. */
async function cmdReceipts(client: LogiosClient, flags: GlobalFlags, arg?: string): Promise<void> {
  const limit = arg ? parseSlot(arg) : 10;
  const receipts = await client.receipts(limit);

  if (flags.json) return printJson(receipts);

  header(`Receipts (latest ${receipts.length})`);
  for (const r of receipts) {
    console.log(
      `${c.cyan("#" + r.slot)}  ${c.gray(short(r.signature))}  ${c.dim(r.createdAt)}`,
    );
    console.log(`  ${r.decision}`);
  }
}

/** `logios explain [slot]` — plain-English narration of a slot (latest if omitted). */
async function cmdExplain(client: LogiosClient, flags: GlobalFlags, arg?: string): Promise<void> {
  const slot = arg ? parseSlot(arg) : undefined;
  const ex = await client.explain(slot);

  if (flags.json) return printJson(ex);

  header(`Slot #${ex.slot}`);
  row("blockhash", c.gray(ex.blockhash));
  row("txns", String(ex.txns));
  console.log();
  console.log(c.italic(wrapText(ex.narration, 76)));
}

/** `logios watch` — poll the latest slot every 2s and stream new ones. */
async function cmdWatch(client: LogiosClient, flags: GlobalFlags): Promise<void> {
  if (flags.json) {
    console.error(c.yellow("watch streams to a TTY; --json is ignored"));
  }
  console.log(c.dim("watching for new slots — ctrl-c to stop\n"));

  let lastSlot = -1;
  const tick = async () => {
    try {
      const block = await client.latestBlock();
      if (block.slot !== lastSlot) {
        lastSlot = block.slot;
        const ts = new Date().toLocaleTimeString("en-US", { hour12: false });
        console.log(
          `${c.gray(ts)}  ${c.cyan("#" + block.slot)}  ` +
            `${c.bold(String(block.txns))} txns  ` +
            `${c.dim(block.computeUnits.toLocaleString("en-US") + " CU")}  ` +
            `${c.gray(short(block.blockhash))}`,
        );
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error(c.red(`  poll failed: ${msg}`));
    }
  };

  await tick();
  const interval = setInterval(tick, 2000);
  // Keep the process alive until interrupted.
  process.on("SIGINT", () => {
    clearInterval(interval);
    console.log(c.dim("\nstopped."));
    process.exit(0);
  });
}

/**
 * `logios faucet <wallet>` — request devnet SOL for a base58 wallet via the
 * Solana JSON-RPC `requestAirdrop` method. This is the one command that hits
 * the live RPC rather than the read API.
 */
async function cmdFaucet(flags: GlobalFlags, wallet?: string): Promise<void> {
  if (!wallet) {
    throw new Error("usage: logios faucet <wallet-pubkey>");
  }
  if (!isBase58(wallet) || wallet.length < 32 || wallet.length > 44) {
    throw new Error(`'${wallet}' does not look like a base58 Solana pubkey`);
  }

  const rpc = process.env.LOGIOS_RPC ?? DEFAULT_RPC;
  const oneSol = 1_000_000_000; // lamports

  const res = await fetch(rpc, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "requestAirdrop",
      params: [wallet, oneSol],
    }),
  });

  const payload = (await res.json()) as {
    result?: string;
    error?: { message?: string };
  };

  if (flags.json) return printJson(payload);

  if (payload.error) {
    throw new Error(`faucet declined: ${payload.error.message ?? "unknown RPC error"}`);
  }

  header("Faucet");
  row("wallet", c.gray(wallet));
  row("amount", "1 SOL (1,000,000,000 lamports)");
  row("rpc", c.dim(rpc));
  row("signature", c.green(payload.result ?? "(pending)"));
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

function header(title: string): void {
  console.log(c.bold(c.magenta(title)));
  console.log(c.gray("─".repeat(Math.min(title.length, 60))));
}

function row(label: string, value: string): void {
  console.log(`${c.dim((label + ":").padEnd(16))}${value}`);
}

function printJson(data: unknown): void {
  console.log(JSON.stringify(data, null, 2));
}

function short(s: string): string {
  return s.length > 16 ? `${s.slice(0, 8)}…${s.slice(-6)}` : s;
}

function wrapText(text: string, width: number): string {
  const words = text.split(/\s+/);
  const lines: string[] = [];
  let line = "";
  for (const w of words) {
    if ((line + " " + w).trim().length > width) {
      lines.push(line.trim());
      line = w;
    } else {
      line += " " + w;
    }
  }
  if (line.trim()) lines.push(line.trim());
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Arg helpers
// ---------------------------------------------------------------------------

function hasFlag(argv: string[], ...names: string[]): boolean {
  return argv.some((a) => names.includes(a));
}

/** Read and remove an `--opt value` pair from argv in place. */
function takeOption(argv: string[], name: string): string | undefined {
  const i = argv.indexOf(name);
  if (i === -1) return undefined;
  const val = argv[i + 1];
  if (val === undefined) throw new Error(`${name} requires a value`);
  argv.splice(i, 2);
  return val;
}

function parseSlot(arg: string): number {
  const n = Number(arg);
  if (!Number.isInteger(n) || n < 0) {
    throw new Error(`expected a non-negative integer, got '${arg}'`);
  }
  return n;
}

function isBase58(s: string): boolean {
  return /^[1-9A-HJ-NP-Za-km-z]+$/.test(s);
}

function fail(err: unknown): never {
  if (err instanceof LogiosApiError) {
    console.error(c.red(`API error (${err.status}) on ${err.path}: ${err.message}`));
  } else if (err instanceof Error) {
    console.error(c.red(`error: ${err.message}`));
  } else {
    console.error(c.red(`error: ${String(err)}`));
  }
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Help
// ---------------------------------------------------------------------------

function printHelp(): void {
  const { bold, cyan, dim, magenta } = c;
  console.log(`${bold(magenta("logios"))} ${dim("v" + PKG_VERSION)} — explore the chain that writes itself

${bold("USAGE")}
  logios <command> [args] [flags]

${bold("COMMANDS")}
  ${cyan("status")}            Network counters, agent status, and the latest slot
  ${cyan("slot")} ${dim("[n]")}        Show a slot (latest if omitted) with its narration
  ${cyan("receipts")} ${dim("[n]")}    List the latest decision receipts (default 10)
  ${cyan("explain")} ${dim("[slot]")}  Plain-English narration of a slot (latest if omitted)
  ${cyan("watch")}             Stream new slots, polling every 2s
  ${cyan("faucet")} ${dim("<wallet>")} Request 1 devnet SOL for a base58 wallet

${bold("FLAGS")}
  ${dim("--url <origin>")}     API origin (default https://hermes-labs.xyz)
  ${dim("--json")}            Emit raw JSON instead of formatted output
  ${dim("-h, --help")}        Show this help
  ${dim("-V, --version")}     Print version

${bold("ENV")}
  ${dim("LOGIOS_RPC")}        Solana RPC for faucet (default ${DEFAULT_RPC})
  ${dim("NO_COLOR")}          Disable colored output

${bold("EXAMPLES")}
  logios status
  logios slot
  logios slot 4542500
  logios explain 4542500
  logios receipts 5 --json
  logios watch
  logios faucet 9KdSJ4jDeG8YaWfAhazQTWgsQHX7UHZPtF6bkYS6krFM
`);
}
