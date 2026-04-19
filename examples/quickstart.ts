/**
 * quickstart.ts — connect to Logios, print the latest slot and its narration.
 *
 *   npm install @logios/sdk
 *   npx tsx examples/quickstart.ts
 */

import { LogiosClient } from "@logios/sdk";

async function main(): Promise<void> {
  const logios = new LogiosClient(); // https://hermes-labs.xyz

  const stats = await logios.stats();
  console.log("Logios — the chain that writes itself");
  console.log(`  height     ${stats.blockHeight}`);
  console.log(`  commits    ${stats.commits}`);
  console.log(`  tps        ${stats.tps}`);
  console.log(`  validators ${stats.validators}`);
  console.log(`  genesis    ${stats.genesisAt}`);

  const agent = await logios.agent();
  console.log(`\nagent ${agent.status} (${agent.version}) — ${agent.task}`);

  const block = await logios.latestBlock();
  console.log(`\nlatest slot #${block.slot}`);
  console.log(`  blockhash     ${block.blockhash}`);
  console.log(`  txns          ${block.txns}`);
  console.log(`  compute units ${block.computeUnits.toLocaleString("en-US")}`);

  const { narration } = await logios.explain(block.slot);
  console.log(`\n  "${narration}"`);
}

main().catch((err) => {
  console.error("quickstart failed:", err);
  process.exit(1);
});
