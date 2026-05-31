# Hermes RFCs

Design decisions for Hermes land here before they land in code. An RFC captures
the problem, the chosen design, the trade-offs we accepted, and what is still
open. Code that implements an RFC references it; code that changes a shipped
design amends the RFC.

## Status values

- **Draft** — under discussion, not yet agreed.
- **Accepted** — agreed and implemented (or being implemented).
- **Planned** — agreed in principle, not yet scheduled for implementation.
- **Superseded** — replaced by a later RFC (linked).

## Index

| RFC | Title | Status |
| --- | --- | --- |
| [0001](RFC-0001-autonomous-block-authoring.md) | Autonomous block authoring | Accepted |
| [0002](RFC-0002-svm-execution-and-compute-budget.md) | SVM execution and the compute budget | Accepted |
| [0003](RFC-0003-decision-receipts.md) | Decision receipts | Accepted |
| [0004](RFC-0004-slashing-and-jail.md) | Slashing and jail | Accepted |
| [0005](RFC-0005-priority-fee-market.md) | Priority-fee market | Accepted |
| [0006](RFC-0006-anchor-idl-to-sbpf.md) | Anchor IDL to sBPF | Planned |

## Writing an RFC

Copy the structure of an existing RFC: **Summary · Motivation · Design ·
Trade-offs · Open questions · Status**. Number it with the next free integer,
open a PR against `develop`, and link it from this table. Keep it terse and
honest about what is unsolved.
