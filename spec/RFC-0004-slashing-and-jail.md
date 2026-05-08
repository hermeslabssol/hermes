# RFC-0004: Slashing and jail

- **Status:** Accepted
- **Implemented in:** `crates/logios-consensus`
- **Author:** Aris Lefebvre

## Summary

Define the slashing engine: what leader misbehavior is provable from receipts,
how a slashing proof is constructed, the stake penalty applied, and the jail
that removes an offending leader from production for a fixed number of epochs.

## Motivation

A trusted-to-decide leader still must not be trusted to break the rules. The one
violation that breaks a chain is equivocation — two valid slots at one height.
We need a penalty that is (a) provable from data already on the ledger, so no
new trusted oracle is introduced, and (b) automatic, so it does not depend on an
operator noticing.

## Design

**Slashable faults:**

1. **Equivocation** — two receipts with the same `slot` and different
   `account_delta_root` or `parent_blockhash`, both signed by the leader.
2. **Duplicate slot production** — two distinct sealed slots at one height
   attributed to the same leader.

Both reduce to: two valid, leader-signed receipts that cannot both be true.

**Proof.** A slashing proof is the pair of conflicting receipts. Because
receipts are deterministic and signed (RFC-0003), the proof is self-contained:
anyone can verify both signatures against the leader pubkey and observe the
conflict. No external attestation is needed. Proofs are reconstructable from the
ledger alone, which is why the slashing data path was moved onto the ledger in
v0.5.0.

**Penalty.** On a verified proof the engine applies a stake penalty to the
offending leader and records the slash with a reference to the proof.

**Jail.** A slashed leader is jailed: removed from slot production for a fixed
number of epochs. The jail counter is tracked in epochs (`logios-primitives`
`Epoch`). `GET /v1/leader` exposes current jail status. Release is automatic
when the jail epoch count elapses; there is no manual unjail in this RFC.

**No false positives.** The engine slashes only on a verifiable proof. A node
that cannot verify both signatures discards the alleged proof. Liveness faults
(a leader that simply stalls) are *not* slashable here — only provable safety
violations are.

## Trade-offs

- **Safety faults only.** We slash equivocation and duplicate production, not
  downtime. Downtime is a liveness problem better handled by rotation, which is
  out of scope until multi-leader exists. This keeps slashing free of false
  positives at the cost of not penalizing an idle leader.
- **Fixed jail duration.** Simple and predictable. It does not escalate for
  repeat offenders; escalation is deferred.
- **Single-leader reality.** On devnet there is one leader, so a slash today is
  effectively a halt until release. Accepted: the engine is built now so it is
  battle-tested before multi-leader makes it routine.

## Open questions

- Should repeat equivocation escalate the jail duration or the stake penalty?
- Who, if anyone, can submit a slashing proof in a multi-leader world, and is
  there a reward for doing so?
- How does jail interact with self-shipped upgrades — can a jailed leader's
  pending upgrade still be applied by its successor?
