# Hermes on-chain programs — Hermes Labs

Two Anchor (Solana / SVM) programs that back **Hermes, the chain that writes
itself**. Hermes authors every slot with an autonomous agent and signs a
decision receipt; these programs let the agent anchor those receipts on-chain
and let testers obtain the project's $LABS SPL token.

| Program | Program ID | Purpose |
|---|---|---|
| `receipt-registry` | `ATNjhU9qwjziAcnS5tcrPPFcrszB9RevcDs9MJkf6MH2` | Append-only registry of per-slot agent **decision receipts**. |
| `hermes-faucet` | `AvPqHdw2HCY8RAEHVqpRiE98SaQENAuj6xjnH8nvWbF2` | Testnet **$LABS** SPL faucet with a per-wallet cooldown. |

## `receipt-registry`

For every slot, the Hermes agent commits a `Receipt` PDA describing the block it
authored: the slot blockhash, transaction count, compute units consumed, and a
SHA-256 commitment to its natural-language narration of the decision. A
singleton `RegistryConfig` PDA names the agent authority and tracks the slot
watermark + receipt count.

Instructions:

- `initialize(authority)` — create the config and bind the Hermes agent key.
- `commit_receipt(slot, blockhash, txns, compute_units, narration_hash)` —
  anchor one receipt. Only the registry authority may call it; slots must be
  strictly monotonic; reported compute units are capped at the per-slot Sealevel
  budget of **48,000,000 CU**. Emits `ReceiptCommitted`.

PDAs: `config = [b"config"]`, `receipt = [b"receipt", slot_le_bytes]`.

Errors: `Unauthorized`, `NonMonotonicSlot`, `ComputeBudgetExceeded`.

## `hermes-faucet`

Mints capped amounts of testnet $LABS to any caller via a `mint_to` CPI into
the SPL Token program, signed by the program's `mint_authority` PDA. A `Claim`
PDA per wallet records the last drip slot to enforce a **~30-second cooldown**
(`75` slots at ~400ms/slot by default).

Instructions:

- `initialize(cooldown_slots)` — bind the faucet to a $LABS mint and set the
  cooldown.
- `drip(amount)` — mint `amount` $LABS to the caller; reverts while the
  cooldown is active. Emits `Dripped`.

PDAs: `config = [b"faucet"]`, `mint_authority = [b"mint_auth"]`,
`claim = [b"claim", wallet]`.

Errors: `CooldownActive`, `InvalidAmount`, `MintMismatch`, `OwnerMismatch`.

## Build & test

```bash
anchor build
anchor test            # runs tests/anchor/*.ts against the configured cluster
```
