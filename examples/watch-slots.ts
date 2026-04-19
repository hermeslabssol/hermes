/**
 * watch-slots.ts — poll the head of the chain and print each new sealed slot.
 *
 *   npx tsx examples/watch-slots.ts
 *
 * Logios seals a slot every ~2s, so we poll on that cadence and only print
 * when the head advances. Ctrl-C to stop.
 */

import { LogiosClient } from "@logios/sdk";

const logios = new LogiosClient();
const POLL_MS = 2000;

let lastSlot = -1;

async function poll(): Promise<void> {
  try {
    const block = await logios.latestBlock();
    if (block.slot === lastSlot) return;
    lastSlot = block.slot;

    // Narrate the freshly sealed slot in one line.
    const { narration } = await logios.explain(block.slot);
    const ts = new Date().toISOString();
    console.log(
      `[${ts}] #${block.slot}  ${block.txns} txns  ` +
        `${block.computeUnits.toLocaleString("en-US")} CU`,
    );
    console.log(`           ${narration}`);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`poll failed: ${msg}`);
  }
}

console.log("watching Logios — ctrl-c to stop\n");
void poll();
const interval = setInterval(() => void poll(), POLL_MS);

process.on("SIGINT", () => {
  clearInterval(interval);
  console.log("\nstopped.");
  process.exit(0);
});
