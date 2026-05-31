# RFC-0003: Decision receipts

- **Status:** Accepted
- **Implemented in:** `crates/hermes-ledger`, `programs/receipt-registry`
- **Author:** Aris Lefebvre

## Summary

Define the decision receipt: the signed, deterministic record the agent emits
for every sealed slot. Specify its canonical byte layout, the base58 Ed25519
signature over it, how it is appended to the ledger, and how it is anchored
on-chain by the `receipt-registry` program.

## Motivation

An autonomous chain is only trustworthy if its decisions are attributable and
verifiable. A receipt turns "the agent did something this slot" into a signed
artifact anyone can check from chain state alone. Receipts are the audit trail
that makes the whole system legible.

## Design

A receipt has a **header** and a **body**.

**Header** (cheap to verify, light-client friendly):

- `slot` — the slot height (u64).
- `parent_blockhash` — base58 hash of slot `n-1`.
- `leader` — base58 pubkey of the authoring leader.

**Body** (the decision detail):

- `decision_summary` — transactions ordered and any upgrade shipped.
- `compute_units` — total CU consumed against the per-slot budget (u64).
- `account_delta_root` — a commitment over the sealed account writes.

**Canonical bytes.** Header and body are serialized in a fixed field order with
fixed-width integers (little-endian) and length-prefixed variable fields. The
layout is versioned by a leading `schema_version` byte so it can evolve without
breaking historical verification. Every node produces identical canonical bytes
for the same sealed slot.

**Signature.** The leader signs the canonical bytes with Ed25519. The signature
is stored and displayed in base58, never hex. Verification recomputes the
canonical bytes from the sealed slot and checks the signature against `leader`.

**Ledger.** `hermes-ledger` appends one receipt per sealed slot. The log is
append-only; a receipt is never mutated. Receipts are addressable by slot height
and by base58 signature.

**On-chain anchor.** The `receipt-registry` Anchor program stores the base58
signature (and slot) on-chain so the existence and ordering of a receipt is
independently verifiable from account state, not only from a node's local
ledger. The program checks that the writer is the registered leader.

**Read access.** `GET /v1/receipt/{sig}` returns a receipt by signature;
`GET /v1/receipts?slot={n}&limit={k}` walks backward from a slot.

## Trade-offs

- **Header/body split.** Adds a little serialization complexity but lets light
  clients verify authorship cheaply without pulling the full body. Worth it.
- **Account-delta commitment, not full deltas, in the receipt.** Keeps receipts
  small; full deltas remain reconstructable from the slot. The commitment must
  be collision-resistant for this to hold (we use SHA-256).
- **On-chain anchor duplicates ledger data.** The signature lives both in the
  ledger and on-chain. The redundancy is the point: it removes the local ledger
  as a trust dependency for proving a receipt exists.

## Open questions

- Should the receipt carry a structured, machine-readable rationale rather than
  a free-form `decision_summary`, to make agent behavior queryable?
- What is the right commitment scheme for `account_delta_root` once ZK receipt
  proofs (a planned line) need to open it succinctly?
- Do upgrade slots warrant extra body fields (program id, build hash) versus
  encoding them inside `decision_summary`?
