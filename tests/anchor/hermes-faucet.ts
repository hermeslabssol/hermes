import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { HermesFaucet } from "../../target/types/hermes_faucet";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  getAccount,
  setAuthority,
  AuthorityType,
} from "@solana/spl-token";
import { assert } from "chai";

/**
 * Tests for the Logios `hermes-faucet` program.
 *
 * Covers: initialize, a successful $HERMES drip (balance + Claim state),
 * and the CooldownActive revert when dripping again too soon.
 */
describe("hermes-faucet", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.HermesFaucet as Program<HermesFaucet>;
  const payer = (provider.wallet as anchor.Wallet).payer;

  // Use a large cooldown so the second drip is guaranteed to be blocked.
  const COOLDOWN_SLOTS = 1_000_000;
  const DRIP_AMOUNT = new BN(500_000_000);

  let mint: PublicKey;

  // PDA: config = [b"faucet"]
  const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("faucet")],
    program.programId
  );

  // PDA: mint_authority = [b"mint_auth"]
  const [mintAuthPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("mint_auth")],
    program.programId
  );

  const claimPda = (wallet: PublicKey): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("claim"), wallet.toBuffer()],
      program.programId
    )[0];

  before(async () => {
    // Create the $HERMES mint with the payer as temporary mint authority...
    mint = await createMint(
      provider.connection,
      payer,
      payer.publicKey,
      null,
      9
    );
    // ...then hand mint authority to the faucet PDA so `drip` can sign mint_to.
    await setAuthority(
      provider.connection,
      payer,
      mint,
      payer,
      AuthorityType.MintTokens,
      mintAuthPda
    );
  });

  it("initializes the faucet for the $HERMES mint", async () => {
    await program.methods
      .initialize(new BN(COOLDOWN_SLOTS))
      .accounts({
        config: configPda,
        mint,
        mintAuthority: mintAuthPda,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const config = await program.account.faucetConfig.fetch(configPda);
    assert.ok(config.mint.equals(mint));
    assert.equal(config.cooldownSlots.toNumber(), COOLDOWN_SLOTS);
  });

  it("drips $HERMES and records the claim", async () => {
    const ata = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      mint,
      payer.publicKey
    );

    await program.methods
      .drip(DRIP_AMOUNT)
      .accounts({
        config: configPda,
        mint,
        mintAuthority: mintAuthPda,
        claim: claimPda(payer.publicKey),
        recipientToken: ata.address,
        wallet: payer.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const acct = await getAccount(provider.connection, ata.address);
    assert.equal(acct.amount.toString(), DRIP_AMOUNT.toString());

    const claim = await program.account.claim.fetch(claimPda(payer.publicKey));
    assert.ok(claim.wallet.equals(payer.publicKey));
    assert.isAbove(claim.lastClaimSlot.toNumber(), 0);
  });

  it("rejects a second drip during cooldown (CooldownActive)", async () => {
    const ata = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      mint,
      payer.publicKey
    );

    try {
      await program.methods
        .drip(DRIP_AMOUNT)
        .accounts({
          config: configPda,
          mint,
          mintAuthority: mintAuthPda,
          claim: claimPda(payer.publicKey),
          recipientToken: ata.address,
          wallet: payer.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      assert.fail("expected CooldownActive");
    } catch (err: any) {
      assert.include(err.toString(), "CooldownActive");
    }
  });

  it("rejects a zero-amount drip (InvalidAmount)", async () => {
    const fresh = Keypair.generate();
    const sig = await provider.connection.requestAirdrop(
      fresh.publicKey,
      anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig, "confirmed");

    const ata = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      mint,
      fresh.publicKey
    );

    try {
      await program.methods
        .drip(new BN(0))
        .accounts({
          config: configPda,
          mint,
          mintAuthority: mintAuthPda,
          claim: claimPda(fresh.publicKey),
          recipientToken: ata.address,
          wallet: fresh.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([fresh])
        .rpc();
      assert.fail("expected InvalidAmount");
    } catch (err: any) {
      assert.include(err.toString(), "InvalidAmount");
    }
  });
});
