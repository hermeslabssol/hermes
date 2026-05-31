# RFC-0005: Priority-fee market

- **Status:** Accepted
- **Implemented in:** `crates/hermes-consensus`, `crates/hermes-runtime`
- **Author:** Niko Vasilakis

## Summary

Specify how the autonomous leader orders the per-slot transaction batch by
priority fee, denominated in lamports, under the fixed compute budget. Define
what a priority fee is, how ties break, and how ordering stays compatible with
deterministic sealing.

## Motivation

Block space is scarce: a slot is capped at 48,000,000 CU (RFC-0002). When the
pending pool exceeds what fits, the leader needs a principled, predictable rule
for what to include and in what order. A lamport priority fee is the Solana-
native mechanism and the one users already understand. Making ordering a stated
rule — not an opaque agent whim — keeps the decision auditable in the receipt.

## Design

**Priority fee.** Each transaction may attach a priority fee in lamports. The
effective priority is the fee per compute unit: `priority_fee / requested_cu`.
Ranking by fee-per-CU, not raw fee, prevents a large cheap transaction from
crowding out several small valuable ones.

**Selection.** The leader fills the slot greedily by descending fee-per-CU,
admitting transactions while the running CU total stays within the per-slot
budget. A transaction that would push the batch over budget is skipped, and the
leader continues down the list (a smaller, cheaper transaction may still fit).

**Tie-break.** Equal fee-per-CU ties break by lower requested CU first, then by
the base58 ordering of the transaction signature. The tie-break is fully
determined by data in the transactions, so every node would order an identical
pool identically.

**Determinism.** Ordering depends only on the pool contents and these rules, not
on wall-clock arrival or node-local state. This matters because the chosen order
is part of the sealed slot and therefore part of the receipt's canonical bytes
(RFC-0003). The receipt's `decision_summary` records the realized ordering.

**Fee disposition.** Priority fees are collected in lamports. Their destination
(burn vs. leader vs. treasury) is a policy knob recorded per slot; the mechanism
here is ordering, not disposition.

## Trade-offs

- **Fee-per-CU over raw fee.** Fairer use of scarce CU, slightly more arithmetic
  per candidate. Cheap and clearly worth it.
- **Greedy, not optimal packing.** True optimal knapsack packing of CU is not
  worth the cost per slot. Greedy-by-density is near-optimal in practice and is
  trivially deterministic, which matters more for verifiability.
- **Deterministic tie-break on signature.** Removes any node-local ambiguity at
  the cost of a tie-break that is not economically meaningful. Acceptable —
  determinism is the priority.

## Open questions

- Should there be a minimum priority fee or a base fee floor to deter spam when
  the pool is below capacity?
- How is fee disposition governed once external validators and a real $HERMES
  economy exist? (Today $HERMES carries no value.)
- Does the agent get any discretion to override pure fee ordering (e.g. to
  prioritize a self-shipped upgrade), and if so, must that override be flagged
  explicitly in the receipt?
