# Consensus

`logios-consensus` runs the single autonomous leader, tracks Tower-BFT votes
(WIP), and enforces slashing and jail. See
[RFC-0001](../spec/RFC-0001-autonomous-block-authoring.md) (authoring) and
[RFC-0004](../spec/RFC-0004-slashing-and-jail.md) (slashing).

## Leader production

On devnet there is exactly one leader: the Logios agent. It runs a loop, one
iteration per slot — observe, decide, stage, execute, seal, sign. A slot is
valid only if it extends the parent height, respects account ownership, stays
within the compute budget, and carries a receipt whose base58 signature
verifies against the leader pubkey.

The protocol checks the *result*, not the agent's reasoning. Validity is
independent of why the agent chose a particular batch.

## Sealing

Sealing fixes the canonical sealed-slot bytes: slot height, parent blockhash,
account deltas, and total compute units. These bytes are what the decision
receipt is signed over, so sealing must be deterministic across nodes.

## Slashing and jail

Two faults are slashable, both reducible to "two valid leader-signed receipts
that cannot both be true":

- **Equivocation** — two receipts at one slot with conflicting state.
- **Duplicate slot production** — two distinct sealed slots at one height.

A slashing proof is just the pair of conflicting receipts; it is self-contained
and reconstructable from the ledger alone. On a verified proof the engine
applies a stake penalty and **jails** the leader for a fixed number of epochs.
Jail status is exposed at `GET /v1/leader`. Release is automatic.

Only provable safety faults are slashed. Liveness faults (a stalled leader) are
not slashable here — that is deferred to multi-leader rotation.

## Status

- Single-leader production: implemented.
- Slashing & jail: implemented, **audited (internal)**.
- Tower-BFT vote tracking beyond one leader: **WIP**.
- Multi-leader rotation across epochs: roadmap.
