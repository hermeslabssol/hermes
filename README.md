<div align="center">

# Hermes

### the chain that writes itself

An autonomous AI agent authors every slot, ships its own code upgrades, and
signs a decision receipt for every choice it makes — live, in public, on Solana.

<br/>

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![build](https://img.shields.io/badge/build-passing-brightgreen.svg)](.github/workflows/ci.yml)
[![stars](https://img.shields.io/badge/stars-71-yellow.svg)](https://github.com/hermeslabssol/hermes/stargazers)
[![contributors](https://img.shields.io/badge/contributors-4-blue.svg)](#maintainers)
[![Solana](https://img.shields.io/badge/Solana-SVM%20%2F%20Sealevel-9945FF.svg)](https://solana.com)
[![Rust](https://img.shields.io/badge/Rust-1.79-orange.svg)](rust-toolchain.toml)
[![version](https://img.shields.io/badge/version-0.5.0-informational.svg)](CHANGELOG.md)
[![X](https://img.shields.io/badge/follow-%40hermeslabsxyz-1DA1F2.svg)](https://x.com/hermeslabsxyz)

[Website](https://hermes-labs.xyz) ·
[Docs](https://hermes-labs.xyz/docs) ·
[Read API](https://hermes-labs.xyz/v1) ·
[RFCs](spec/) ·
[Changelog](CHANGELOG.md)

</div>

---

> [!WARNING]
> **Experimental. Devnet only.** Hermes is a research system. Consensus,
> slashing, and the runtime are under active development and have not been
> audited end to end. The autonomous leader runs unattended on devnet. The
> $LABS SPL token carries no value and confers no rights. Do not point
> anything you cannot lose at this software. **DYOR.**

---

## Why Hermes

Most chains are written by people and run by machines. Hermes inverts the
second half. A single autonomous agent — Hermes — holds the leader slot on
devnet. Each slot, it proposes a block, writes the accounts, builds and
executes the programs under a fixed compute budget, seals the slot, and signs
a **decision receipt** recording what it did and why.

The receipt is the point. Every autonomous choice is attributable, ordered,
and verifiable from the ledger alone. You do not have to trust a description of
what the agent "intends" to do. You read the signed record of what it did.

Hermes is **Solana-native** end to end. It runs the SVM (Sealevel) runtime,
addresses state through accounts owned by programs, meters work in compute
units against a per-slot budget, prices priority in lamports, and identifies
everything — pubkeys, blockhashes, signatures — in base58. There is no second
execution environment bolted on. If you have written a Solana program, the
mental model already fits.

What is genuinely different here:

- **Authorship is autonomous.** The leader is the agent, not a human operator
  pressing deploy. Slot production runs as a loop.
- **Upgrades are self-shipped.** The agent builds program changes
  (`cargo build-sbf` / `anchor build`) and proposes them through the same
  receipt-signed path as ordinary slots.
- **Every decision is receipted.** One signed receipt per sealed slot, anchored
  on-chain by the `receipt-registry` program and served read-only over `/v1`.

## Architecture

Hermes is a small stack of focused crates. Data flows up from primitives to the
public surface; control flows down from the agent into the runtime each slot.

```
                         ┌───────────────────────────────┐
                         │            Hermes              │
                         │      autonomous AI leader      │
                         │  propose · build · seal · sign │
                         └───────────────┬───────────────┘
                                         │ slot plan
                                         ▼
        ┌─────────────────────────────────────────────────────────┐
        │  hermes-runtime                                           │
        │  SVM / Sealevel executor · accounts · programs           │
        │  compute-budget metering  (cap 48,000,000 CU / slot)     │
        └───────────────┬─────────────────────────────────────────┘
                        │ executed batch + CU total
                        ▼
        ┌─────────────────────────────────────────────────────────┐
        │  hermes-consensus                                        │
        │  single-leader slot production · Tower-BFT votes (WIP)   │
        │  slashing + jail on equivocation / duplicate slots       │
        └───────────────┬─────────────────────────────────────────┘
                        │ sealed slot
                        ▼
        ┌─────────────────────────────────────────────────────────┐
        │  hermes-ledger                                           │
        │  append-only log · one signed decision receipt per slot │
        └───────────────┬─────────────────────────────────────────┘
                        │
            ┌───────────┴────────────┐
            ▼                        ▼
   ┌──────────────────┐    ┌────────────────────────┐
   │  /v1 read API    │    │  programs (Anchor)      │
   │  slots ·receipts │    │  receipt-registry       │
   │  health          │    │  hermes-faucet          │
   └────────┬─────────┘    └────────────────────────┘
            │
   ┌────────┴─────────────────────────────┐
   ▼                                       ▼
┌──────────────────┐            ┌────────────────────┐
│  sdk/ts          │            │  sdk/rust + cli    │
│  @hermes/sdk     │            │  hermes-client     │
└──────────────────┘            └────────────────────┘
```

Everything below `hermes-runtime` is built on `hermes-primitives`, the
dependency-light vocabulary crate that defines base58 pubkeys, hashes,
signatures, and the slot / epoch / lamport / compute-unit units.

## How it works

One slot, start to finish:

1. **Propose.** The agent selects the next slot's work: pending transactions
   ordered by lamport priority fee, plus any self-authored program upgrade it
   has decided to ship.
2. **Write accounts.** The runtime stages the account set the batch will read
   and write, enforcing program ownership.
3. **Build.** Program changes are compiled to sBPF (`cargo build-sbf`) and
   Anchor programs via `anchor build`. A failed build aborts the slot plan, not
   the chain.
4. **Execute under budget.** The Sealevel executor runs the batch, metering
   every instruction in compute units. The aggregate may not exceed the
   per-slot budget of 48,000,000 CU.
5. **Seal the slot.** Consensus finalizes the slot, fixing the canonical byte
   layout (parent blockhash, account deltas, CU consumed).
6. **Sign the receipt.** The agent signs a decision receipt over the sealed
   slot. The base58 Ed25519 signature is appended to the ledger and anchored
   on-chain by `receipt-registry`. The receipt is then queryable at
   `/v1/receipt/{sig}`.

If the leader equivocates — two valid slots at one height — the slashing engine
constructs a proof from the receipts, applies the stake penalty, and jails the
leader for a fixed number of epochs.

## Quickstart

Requires Rust 1.79+, the Solana toolchain, Anchor 0.30+, and Node 20+ for the
TypeScript SDK.

```bash
# 1. Clone
git clone https://github.com/hermeslabssol/hermes
cd hermes

# 2. Build the Rust workspace (primitives, runtime, consensus, ledger, sdk, cli)
cargo build

# 3. Run the core test suite
cargo test --all

# 4. Build the on-chain Anchor programs
anchor build

# 5. Run the TypeScript SDK example against devnet
cd sdk/ts
npm ci
npm run build
node examples/stream-slots.js
```

Read the latest sealed slot and its receipt straight from the public API:

```bash
# Latest health + slot height
curl https://hermes-labs.xyz/v1/health

# A specific slot
curl https://hermes-labs.xyz/v1/slot/12345

# A receipt by its base58 signature
curl https://hermes-labs.xyz/v1/receipt/5Hd9...e3Qa
```

Or with the CLI:

```bash
cargo run -p hermes-cli -- slot latest
cargo run -p hermes-cli -- receipt get 5Hd9...e3Qa
```

## The Stack

| Component | Path | What it does |
| --- | --- | --- |
| `hermes-primitives` | `crates/hermes-primitives` | Base58 pubkeys, hashes, Ed25519 signatures; slot / epoch / lamport / compute-unit units and constants. |
| `hermes-runtime` | `crates/hermes-runtime` | SVM / Sealevel executor: accounts, programs, transaction batches, per-slot compute-budget metering. |
| `hermes-consensus` | `crates/hermes-consensus` | Single autonomous leader, Tower-BFT vote tracking (WIP), slashing and jail engine. |
| `hermes-ledger` | `crates/hermes-ledger` | Append-only log of one signed decision receipt per sealed slot. |
| `receipt-registry` | `programs/receipt-registry` | Anchor program anchoring each receipt's base58 signature on-chain. |
| `hermes-faucet` | `programs/hermes-faucet` | Anchor program: rate-limited devnet $LABS drip, one claim per pubkey per epoch. |
| `@hermes/sdk` | `sdk/ts` | TypeScript client for `/v1`: slot streaming, receipt lookup, typed responses. |
| `hermes-client` | `sdk/rust` | Rust client over `/v1` and the on-chain programs. |
| `hermes-cli` | `cli` | Operator and explorer CLI: query slots, fetch receipts, inspect the ledger. |
| examples | `examples` | Runnable samples for both SDKs. |

## Decision Receipts

A **decision receipt** is the signed record the agent emits for every sealed
slot. It is what makes an autonomous chain auditable rather than opaque.

Each receipt contains:

- the **slot** height and the base58 **blockhash** of the parent slot,
- the leader **pubkey** (base58),
- a summary of the decision (transactions ordered, upgrades shipped),
- the **compute units** consumed against the per-slot budget,
- a base58 **Ed25519 signature** over the canonical sealed-slot bytes.

Receipts are deterministic: every node reconstructs the same canonical bytes
and verifies the same signature. They are appended to `hermes-ledger` and
anchored on-chain by the `receipt-registry` Anchor program, so the signature
of any slot is independently checkable from chain state. Read one by signature
at `GET /v1/receipt/{sig}`.

See [RFC-0003](spec/RFC-0003-decision-receipts.md) for the byte layout and the
verification procedure.

## Public API

Read-only. Served at `https://hermes-labs.xyz/v1`. No auth, rate-limited per IP.
JSON responses; all hashes, pubkeys, and signatures are base58.

| Method | Endpoint | Returns |
| --- | --- | --- |
| `GET` | `/v1/health` | Node liveness, current slot height, leader pubkey. |
| `GET` | `/v1/slot/latest` | The most recently sealed slot. |
| `GET` | `/v1/slot/{n}` | Sealed slot `n`: parent blockhash, CU consumed, receipt signature. |
| `GET` | `/v1/receipt/{sig}` | The decision receipt for base58 signature `sig`. |
| `GET` | `/v1/receipts?slot={n}&limit={k}` | Receipts from slot `n` backward, newest first. |
| `GET` | `/v1/leader` | Current leader pubkey and jail status. |
| `GET` | `/v1/budget` | Per-slot compute budget (48,000,000 CU) and recent utilization. |

Full reference: [`docs/api.md`](docs/api.md).

## Security model

Hermes trusts one component on devnet: the autonomous leader's signing key.
Everything else is verified.

- **Authorship is signed.** No slot is valid without a receipt signature that
  verifies against the leader pubkey.
- **Equivocation is punished.** Two valid slots at one height yield a slashing
  proof reconstructable from receipts; the offender loses stake and is jailed
  for a fixed epoch count.
- **Execution is bounded.** The runtime caps each slot at 48,000,000 CU. A slot
  plan that would exceed the budget is rejected before it can seal.
- **Programs check signers and owners.** Anchor programs enforce signer and
  account-owner constraints; the faucet enforces one claim per pubkey per epoch.

### Audit status

| Component | Status |
| --- | --- |
| Slashing & jail (`hermes-consensus`) | Audited (internal) |
| Decision receipts (`hermes-ledger`, `receipt-registry`) | Audited (internal) |
| Runtime / compute budget (`hermes-runtime`) | WIP — not yet audited |
| Tower-BFT vote tracking | WIP |
| ZK receipt proofs | Planned |

Report vulnerabilities per [`SECURITY.md`](SECURITY.md).

## RFCs

Design lands as an RFC before it lands as code. The full index lives in
[`spec/README.md`](spec/README.md).

| RFC | Title | Status |
| --- | --- | --- |
| [0001](spec/RFC-0001-autonomous-block-authoring.md) | Autonomous block authoring | Accepted |
| [0002](spec/RFC-0002-svm-execution-and-compute-budget.md) | SVM execution and the compute budget | Accepted |
| [0003](spec/RFC-0003-decision-receipts.md) | Decision receipts | Accepted |
| [0004](spec/RFC-0004-slashing-and-jail.md) | Slashing and jail | Accepted |
| [0005](spec/RFC-0005-priority-fee-market.md) | Priority-fee market | Accepted |
| [0006](spec/RFC-0006-anchor-idl-to-sbpf.md) | Anchor IDL to sBPF | Planned |

## Roadmap

- [x] `hermes-primitives`: base58 types, slot / lamport / compute-unit units
- [x] SVM / Sealevel executor with per-slot compute-budget metering
- [x] Append-only ledger of signed decision receipts
- [x] `receipt-registry` Anchor program anchoring receipts on-chain
- [x] Slashing and jail on equivocation / duplicate slots
- [x] Priority-fee market ordered by lamports under the CU budget
- [x] `hermes-faucet` devnet $LABS drip
- [ ] Tower-BFT vote tracking beyond a single leader
- [ ] Multi-leader rotation across epochs
- [ ] Self-shipped program upgrades through the receipt path (hardening)
- [ ] ZK proofs over receipts for light-client verification
- [ ] Public testnet with external validators
- [ ] $LABS SPL mint on pump.fun (CA TBA)

## Star history

```
stars
  71 ┤                                          ╭─
     │                                       ╭──╯
     │                                   ╭───╯
  40 ┤                            ╭──────╯
     │                     ╭──────╯
     │            ╭────────╯
   0 ┼────────────╯
     └────┬────┬────┬────┬────┬────┬────┬────
        v0.1 v0.2 v0.3 v0.4 v0.5 …
```

[View on star-history.com →](https://star-history.com/#hermeslabssol/hermes)

## Maintainers

| Name | Area |
| --- | --- |
| **hermeslabssol** | Lead — runtime + consensus |
| **Niko Vasilakis** | Runtime / SVM executor |
| **Aris Lefebvre** | Consensus / Anchor programs |
| **Demetra Soto** | SDK / developer experience |

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md). Substantial changes go through an RFC
in [`spec/`](spec/) first. Conventional commits, `cargo test` + `anchor test`
green, and review by every touched path's code owner.

## Security

See [`SECURITY.md`](SECURITY.md). Report vulnerabilities to
**security@hermes-labs.xyz**, not the public issue tracker.

## License

Apache-2.0. See [`LICENSE`](LICENSE). © 2026 Hermes Labs.

---

<div align="center">

**Hermes** · a Hermes Labs project ·
[hermes-labs.xyz](https://hermes-labs.xyz) ·
[@hermeslabsxyz](https://x.com/hermeslabsxyz)

</div>

<!-- maintained 2026-06-04 -->
