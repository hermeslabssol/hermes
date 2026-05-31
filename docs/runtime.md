# Runtime

`hermes-runtime` is the SVM / Sealevel-style execution engine. It models state
as accounts, runs programs over transaction batches, and meters every
instruction in compute units against a per-slot budget. Full design:
[RFC-0002](../spec/RFC-0002-svm-execution-and-compute-budget.md).

## State model

- **Account** — owner program, lamport balance, data buffer. A program may write
  only accounts it owns.
- **Program** — an sBPF executable (Anchor or raw) invoked by instructions.
- **Instruction** — names a program and the accounts it touches.
- **Transaction** — an ordered list of instructions over a declared account set.
- **Batch** — the ordered transactions the leader includes in a slot.

Ownership is checked at stage time, before execution, so violations fail cheaply
and never reach the executor.

## Compute budget

Compute units are accumulated per instruction, per transaction, and per batch.
The per-slot ceiling lives in `hermes-primitives`:

```rust
pub const MAX_COMPUTE_UNITS_PER_SLOT: u64 = 48_000_000;
```

- A transaction over its declared CU limit fails and rolls back its writes.
- A batch whose aggregate CU would exceed the per-slot budget is rejected; the
  runtime returns the overflow to the leader, which trims the slot plan.
- A slot never seals over budget.

Cross-program invocation charges the callee's CU to the same batch total and
counts against a bounded invocation depth.

## Determinism

Given the same starting account set and the same ordered batch, the runtime
produces identical account deltas and an identical total CU on every node. This
is the property that makes decision receipts independently verifiable.

## Status

The runtime is **WIP and not yet audited** (see the audit table in the README).
The account model, batch execution, and compute-budget metering are implemented;
syscall pricing for agent-introduced programs is an open question in RFC-0002.
