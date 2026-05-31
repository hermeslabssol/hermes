# RFC-0001: Autonomous block authoring

- **Status:** Accepted
- **Implemented in:** `crates/hermes-consensus`, `crates/hermes-runtime`
- **Author:** hermeslabssol

## Summary

Define how an autonomous agent (Hermes) holds the leader slot and authors every
slot on devnet: the production loop, what counts as a valid authored slot, and
the boundary between the agent's decisions and the protocol's verification.

## Motivation

The premise of Hermes is a chain whose blocks are authored by an agent rather
than scheduled by an operator. That only means something if "authored" is
precise. We need a single definition of the slot loop, what the agent is free
to choose, and what the protocol checks regardless of what the agent chose. The
agent is trusted to decide; it is not trusted to violate the rules.

## Design

The leader runs a loop, one iteration per slot:

1. **Observe.** Read the current slot height, the parent blockhash, and the
   pending transaction pool.
2. **Decide.** Select the batch to include and its order, plus any self-authored
   program upgrade to ship this slot. Ordering follows the priority-fee market
   (RFC-0005).
3. **Stage.** Hand the batch to the runtime, which stages the read/write account
   set and enforces program ownership.
4. **Execute.** The runtime runs the batch under the per-slot compute budget
   (RFC-0002). Overflow aborts the slot plan.
5. **Seal.** Consensus fixes the canonical sealed-slot bytes: slot height,
   parent blockhash, account deltas, total compute units.
6. **Sign.** The agent signs a decision receipt (RFC-0003) over the sealed
   bytes and appends it to the ledger.

A slot is **valid** iff: it extends the parent at height `n-1`; its batch
respects account ownership; its aggregate compute units are within budget; and
it carries a receipt whose base58 Ed25519 signature verifies against the
current leader pubkey. Validity is independent of *why* the agent chose the
batch — the protocol checks the result, not the reasoning.

On devnet there is exactly one leader. The signing key is the single trusted
component; see the security model in the README.

## Trade-offs

- **Single leader, no live rotation.** Simplifies the loop and makes receipts
  the only authorship record, at the cost of being a single point of failure
  until multi-leader rotation lands. Accepted for devnet.
- **Trusted leader key.** We do not defend against a compromised leader key in
  this RFC. Equivocation by the holder of the key is addressed by slashing
  (RFC-0004), but key custody is out of scope here.
- **Agent decisions are opaque, results are not.** We deliberately do not try to
  constrain the agent's reasoning. We constrain and record its outputs. This
  keeps the protocol simple and the receipts meaningful.

## Open questions

- How does authorship validity change once more than one leader can produce
  slots within an epoch? (Tracked toward multi-leader rotation.)
- Should a self-shipped upgrade slot carry a distinct receipt type from an
  ordinary transaction slot, or is one receipt schema enough?
- What is the recovery procedure if the leader loop stalls mid-slot before
  sealing?
