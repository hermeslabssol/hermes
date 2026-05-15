# Changelog

All notable changes to Logios are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Logios is pre-1.0. Minor versions may carry breaking changes to the protocol,
the `/v1` API, and the SDKs.

## [Unreleased]

### Added
- RFC-0006 draft: lowering Anchor IDL to sBPF entrypoints (planned).

### Changed
- Tightened compute-budget accounting for cross-program invocation depth.

## [0.5.0] - 2026-06-01

### Added
- `programs/hermes-faucet` Anchor program: rate-limited devnet $HERMES drip,
  one claim per pubkey per epoch.
- Priority-fee market: the leader now orders the per-slot transaction batch by
  lamport priority fee under the 48,000,000 CU budget (RFC-0005).
- `sdk/ts` (`@logios/sdk`) `getReceipt` / `streamSlots` helpers over `/v1`.

### Changed
- Decision receipts now embed the sealed-slot compute-unit total and the
  base58 blockhash of the parent slot.
- Slashing engine moved equivocation proofs to the append-only ledger so a
  proof is reconstructable from receipts alone.

### Fixed
- Leader no longer double-counts a vote landing in the same slot it was cast.

_Credits: Niko Vasilakis (faucet, fee market), Aris Lefebvre (slashing proofs),
Demetra Soto (TS SDK)._

## [0.4.0] - 2026-05-10

### Added
- `crates/logios-consensus` slashing engine: detect equivocation and
  duplicate slot production, apply stake penalty, jail the offending leader
  for a fixed epoch count (RFC-0004).
- `programs/receipt-registry` Anchor program: on-chain anchor for the base58
  receipt signature of each sealed slot (RFC-0003).

### Changed
- Receipts split into a header (slot, parent, leader pubkey) and a body
  (decision summary, CU consumed) so light clients can verify headers cheaply.

_Credits: Aris Lefebvre (slashing, registry), hermeslabssol (consensus core)._

## [0.3.0] - 2026-04-12

### Added
- `crates/logios-ledger`: append-only log of one signed decision receipt per
  sealed slot, with base58 signature verification.
- `/v1` read API: `GET /v1/slot/{n}`, `GET /v1/receipt/{sig}`,
  `GET /v1/health`.

### Changed
- Slot sealing now produces a canonical byte layout for the receipt so the
  Ed25519 signature is deterministic across nodes.

## [0.2.0] - 2026-03-15

### Added
- `crates/logios-runtime`: SVM / Sealevel-style executor with accounts,
  programs, and per-slot compute-budget metering capped at 48,000,000 CU
  (RFC-0002).
- Compute-unit accounting per instruction and per transaction batch.

### Changed
- `Lamports` and `ComputeUnits` moved into `logios-primitives` as newtypes.

## [0.1.0] - 2026-02-20

### Added
- `crates/logios-primitives`: base58 pubkeys, blockhashes, and Ed25519
  signatures; slot, epoch, lamport, and compute-unit units.
- Single autonomous leader authoring one slot at a time on devnet (RFC-0001).
- Workspace scaffold, Apache-2.0 license, CI skeleton.

_Credits: hermeslabssol (primitives, leader loop)._

[Unreleased]: https://github.com/hermeslabssol/logios/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/hermeslabssol/logios/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/hermeslabssol/logios/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/hermeslabssol/logios/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/hermeslabssol/logios/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/hermeslabssol/logios/releases/tag/v0.1.0
