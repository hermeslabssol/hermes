# Multi-leader quorum (WIP)

> Status: design — see issue #14.

Devnet runs a single autonomous leader (`self`). This document tracks the path to a
small leader set voting under Tower-BFT.

## Open questions
- Leader rotation cadence (per-epoch vs per-N-slots)
- Vote aggregation + lockout interaction with the slashing engine
- Equivocation evidence gossip
