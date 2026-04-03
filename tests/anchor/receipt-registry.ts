import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { ReceiptRegistry } from "../../target/types/receipt_registry";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { createHash } from "crypto";
import { assert } from "chai";

/**
 * Tests for the Logios `receipt-registry` program.
 *
 * Covers: initialize, committing a per-slot decision receipt (state + event),
 * and the Unauthorized / NonMonotonicSlot revert paths.
 */
describe("receipt-registry", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .ReceiptRegistry as Program<ReceiptRegistry>;

  // The Logios agent authority that is allowed to commit receipts.
  const agent = Keypair.generate();

  // PDA: config = [b"config"]
  const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  );

  const receiptPda = (slot: number | BN): PublicKey => {
    const slotBn = new BN(slot);
    return PublicKey.findProgramAddressSync(
      [Buffer.from("receipt"), slotBn.toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];
  };

  const sha256 = (s: string): number[] =>
    Array.from(createHash("sha256").update(s).digest());

  before(async () => {
    // Fund the agent so it can pay receipt rent.
    const sig = await provider.connection.requestAirdrop(
      agent.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig, "confirmed");
  });

  it("initializes the registry bound to the Logios agent", async () => {
    await program.methods
      .initialize(agent.publicKey)
      .accounts({
        config: configPda,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const config = await program.account.registryConfig.fetch(configPda);
    assert.ok(config.authority.equals(agent.publicKey));
    assert.equal(config.currentSlot.toNumber(), 0);
    assert.equal(config.totalReceipts.toNumber(), 0);
  });

  it("commits a receipt and emits ReceiptCommitted", async () => {
    const slot = 100;
    const blockhash = Array.from({ length: 32 }, (_, i) => (i * 7) % 256);
    const txns = 1280;
    const computeUnits = new BN(12_500_000);
    const narrationHash = sha256(
      "Logios authored slot 100: prioritized 1280 txns, deferred 3 high-CU CPIs."
    );

    let captured: any = null;
    const listener = program.addEventListener(
      "receiptCommitted",
      (ev) => (captured = ev)
    );

    await program.methods
      .commitReceipt(
        new BN(slot),
        blockhash,
        txns,
        computeUnits,
        narrationHash
      )
      .accounts({
        config: configPda,
        receipt: receiptPda(slot),
        authority: agent.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([agent])
      .rpc();

    // Give the listener a tick to fire.
    await new Promise((r) => setTimeout(r, 1500));
    await program.removeEventListener(listener);

    const receipt = await program.account.receipt.fetch(receiptPda(slot));
    assert.equal(receipt.slot.toNumber(), slot);
    assert.equal(receipt.txns, txns);
    assert.equal(receipt.computeUnits.toNumber(), computeUnits.toNumber());
    assert.deepEqual(Array.from(receipt.blockhash), blockhash);
    assert.deepEqual(Array.from(receipt.narrationHash), narrationHash);
    assert.ok(receipt.authority.equals(agent.publicKey));

    const config = await program.account.registryConfig.fetch(configPda);
    assert.equal(config.currentSlot.toNumber(), slot);
    assert.equal(config.totalReceipts.toNumber(), 1);

    assert.isNotNull(captured, "ReceiptCommitted event should have fired");
    assert.equal(captured.slot.toNumber(), slot);
    assert.equal(captured.totalReceipts.toNumber(), 1);
  });

  it("rejects a non-authority signer (Unauthorized)", async () => {
    const impostor = Keypair.generate();
    const sig = await provider.connection.requestAirdrop(
      impostor.publicKey,
      anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig, "confirmed");

    const slot = 200;
    try {
      await program.methods
        .commitReceipt(
          new BN(slot),
          Array(32).fill(1),
          1,
          new BN(1),
          sha256("impostor attempt")
        )
        .accounts({
          config: configPda,
          receipt: receiptPda(slot),
          authority: impostor.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([impostor])
        .rpc();
      assert.fail("expected Unauthorized");
    } catch (err: any) {
      assert.include(err.toString(), "Unauthorized");
    }
  });

  it("rejects a non-monotonic slot (NonMonotonicSlot)", async () => {
    // current_slot is now 100; committing slot 50 must revert.
    const slot = 50;
    try {
      await program.methods
        .commitReceipt(
          new BN(slot),
          Array(32).fill(2),
          1,
          new BN(1),
          sha256("stale slot")
        )
        .accounts({
          config: configPda,
          receipt: receiptPda(slot),
          authority: agent.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([agent])
        .rpc();
      assert.fail("expected NonMonotonicSlot");
    } catch (err: any) {
      assert.include(err.toString(), "NonMonotonicSlot");
    }
  });

  it("rejects compute units over the Sealevel cap (ComputeBudgetExceeded)", async () => {
    const slot = 300;
    try {
      await program.methods
        .commitReceipt(
          new BN(slot),
          Array(32).fill(3),
          1,
          new BN(48_000_001), // one over MAX_BLOCK_COMPUTE_UNITS
          sha256("over budget")
        )
        .accounts({
          config: configPda,
          receipt: receiptPda(slot),
          authority: agent.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([agent])
        .rpc();
      assert.fail("expected ComputeBudgetExceeded");
    } catch (err: any) {
      assert.include(err.toString(), "ComputeBudgetExceeded");
    }
  });
});
