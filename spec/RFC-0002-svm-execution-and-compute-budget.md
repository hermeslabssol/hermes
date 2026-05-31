# RFC-0002: SVM execution and the compute budget

- **Status:** Accepted
- **Implemented in:** `crates/hermes-runtime`
- **Author:** Niko Vasilakis

## Summary

Specify the Hermes execution environment: the SVM (Sealevel) model of accounts
and programs, how instructions are metered in compute units, and the per-slot
compute budget that bounds a sealed slot at 48,000,000 CU.

## Motivation

Autonomous authorship needs a bounded execution model. If the agent could
include a batch of unbounded cost, a single slot could stall the loop or be used
to grief the chain. Solana's Sealevel runtime already gives us the right shape:
parallelizable account access and a compute-unit meter. We adopt it directly
rather than inventing a new VM, so Solana program authors keep their model.

## Design

**State** is a set of accounts. Each account has an owner program, a lamport
balance, and a data buffer. A program may write only accounts it owns; the
runtime enforces this at stage time, before execution.

**Programs** are sBPF executables (Anchor or raw). They are invoked by
instructions, which name the program and the accounts they touch.

**A transaction** is an ordered list of instructions over a declared account
set. A **batch** is the ordered set of transactions the leader includes in a
slot.

**Metering.** Every instruction consumes compute units. The runtime accumulates
CU per instruction, per transaction, and per batch. The constants live in
`hermes-primitives`:

- `MAX_COMPUTE_UNITS_PER_SLOT = 48_000_000` — the per-slot ceiling.

A transaction that exceeds its declared CU limit fails and rolls back its
account writes. A batch whose aggregate CU would exceed the per-slot budget is
rejected: the runtime returns the overflow to the leader, which trims the slot
plan (RFC-0001 step 4). A slot never seals over budget.

**Cross-program invocation** charges the callee's CU to the same batch total and
counts against invocation depth. Depth and CU are both bounded.

**Determinism.** Given the same starting account set and the same ordered batch,
the runtime produces the same account deltas and the same total CU on every
node. This is what lets receipts (RFC-0003) be verified independently.

## Trade-offs

- **Fixed 48,000,000 CU budget.** Mirrors Solana's block compute limit for
  familiarity. A fixed budget is simple to reason about but does not adapt to
  load; a dynamic budget is deferred.
- **Ownership checked at stage time.** Catches violations before execution at
  the cost of a staging pass over the declared account set. Worth it — it keeps
  execution failures cheap and keeps the deltas clean for sealing.
- **Adopting Sealevel wholesale.** We inherit its strengths and its sharp edges
  (account-conflict serialization). We judged compatibility with existing Solana
  tooling more valuable than a bespoke model.

## Open questions

- Should the per-slot budget become dynamic once external validators join, and
  if so, governed by what?
- How are CU costs for novel syscalls priced, and who sets the schedule when the
  agent itself can ship a program that introduces one?
- Do we need a separate, smaller budget specifically for self-shipped upgrade
  slots to bound build-and-deploy cost?
