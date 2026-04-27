# Roadmap

Logios is pre-1.0 and devnet-only. Terms are Solana-native throughout: slots,
epochs, compute units, lamports, leaders.

## Shipped

- [x] `logios-primitives` — base58 types; slot / epoch / lamport / compute-unit
  units and constants.
- [x] SVM / Sealevel executor with per-slot compute-budget metering (48,000,000
  CU cap).
- [x] Append-only ledger of signed decision receipts, one per sealed slot.
- [x] `receipt-registry` Anchor program anchoring receipts on-chain.
- [x] Slashing and jail on equivocation / duplicate slot production.
- [x] Priority-fee market: batch ordered by lamport fee-per-CU under the budget.
- [x] `hermes-faucet` Anchor program: devnet $HERMES drip, one claim per pubkey
  per epoch.
- [x] `/v1` read API and the TS / Rust SDKs.

## In progress

- [ ] Tower-BFT vote tracking beyond a single leader.
- [ ] Self-shipped program upgrades through the receipt path (hardening).

## Planned

- [ ] Multi-leader rotation across epochs.
- [ ] Anchor IDL → sBPF upgrade pipeline ([RFC-0006](../spec/RFC-0006-anchor-idl-to-sbpf.md)).
- [ ] ZK proofs over receipts for succinct light-client verification.
- [ ] Public testnet with external validators.
- [ ] $HERMES SPL mint on pump.fun (CA TBA — token carries no value).

## Out of scope (for now)

- Mainnet. Logios is a research system; there is no mainnet target date.
- Defending against a compromised leader signing key. The key is a trusted
  component on devnet.
- An economic model for $HERMES. The token confers no rights and no value today.

Dates are intentionally omitted. Progress is visible slot-by-slot at
[hermes-labs.xyz](https://hermes-labs.xyz) and in the
[changelog](../CHANGELOG.md).
