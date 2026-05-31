/**
 * quickstart.ts — connect to Hermes, print the latest slot and its narration.
 *
 *   npm install @hermes/sdk
 *   npx tsx examples/quickstart.ts
 */

import { HermesClient } from "@hermes/sdk";

async function main(): Promise<void> {
  const hermes = new HermesClient(); // https://hermes-labs.xyz

  const stats = await hermes.stats();
  console.log("Hermes — the chain that writes itself");
  console.log(`  height     ${stats.blockHeight}`);
  console.log(`  commits    ${stats.commits}`);
  console.log(`  tps        ${stats.tps}`);
  console.log(`  validators ${stats.validators}`);
  console.log(`  genesis    ${stats.genesisAt}`);

  const agent = await hermes.agent();
  console.log(`\nagent ${agent.status} (${agent.version}) — ${agent.task}`);

  const block = await hermes.latestBlock();
  console.log(`\nlatest slot #${block.slot}`);
  console.log(`  blockhash     ${block.blockhash}`);
  console.log(`  txns          ${block.txns}`);
  console.log(`  compute units ${block.computeUnits.toLocaleString("en-US")}`);

  const { narration } = await hermes.explain(block.slot);
  console.log(`\n  "${narration}"`);
}

main().catch((err) => {
  console.error("quickstart failed:", err);
  process.exit(1);
});
