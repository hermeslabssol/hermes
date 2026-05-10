# RFC-0006: Anchor IDL to sBPF

- **Status:** Planned
- **Implemented in:** _(not yet)_
- **Author:** Demetra Soto

## Summary

Propose a pipeline that takes an Anchor program's IDL and produces the artifacts
the autonomous leader needs to ship and invoke a self-authored sBPF program
upgrade: typed instruction builders, account constraints, and the build hash
recorded in the upgrade receipt. This RFC is **planned** — design only, no
implementation yet.

## Motivation

When Logios ships its own program upgrades (RFC-0001), it must do so through a
path that is as legible as ordinary slots. Today a self-shipped upgrade is built
with `anchor build` and invoked by hand-rolled instruction encoding. We want the
agent to derive instruction builders and account constraints directly from the
Anchor IDL, so the upgrade it ships and the calls it makes are provably tied to
a known IDL and build hash. This shrinks the trusted surface of self-upgrades.

## Design (proposed)

The pipeline has three stages:

1. **IDL ingest.** Parse the Anchor IDL emitted by `anchor build`. Extract
   instructions, account metas with their signer/owner constraints, and the
   program id.
2. **Builder generation.** Emit typed instruction builders (for `sdk/rust` and
   `@logios/sdk`) that the leader and clients use instead of hand-encoding. The
   builders carry the account constraints from the IDL so a malformed call fails
   before it reaches the runtime.
3. **Upgrade binding.** Compute a build hash over the sBPF artifact and bind it,
   with the IDL hash, into the upgrade slot's receipt body. An upgrade receipt
   then proves *which* program, built from *which* IDL, was shipped.

Open verification idea: a light client could check that an on-chain program's id
and IDL hash match what an upgrade receipt claims, without trusting the leader's
self-report.

## Trade-offs (anticipated)

- **IDL as source of truth.** Convenient and already produced by Anchor, but the
  IDL is only as trustworthy as the build that emitted it. The build hash
  binding is meant to close that gap; whether it fully does is open.
- **Generated builders vs. hand-written.** Generation removes a class of
  encoding bugs but adds a codegen step to the build and a tool to maintain.
- **Scope creep risk.** This touches both SDKs, the runtime invocation path, and
  the receipt schema. It is deliberately Planned, not Draft-for-merge, until the
  receipt-schema impact (RFC-0003 open question on upgrade fields) is settled.

## Open questions

- Should the build hash cover only the sBPF artifact, or the IDL and source tree
  as well, to make upgrades fully reproducible?
- Where does codegen run in CI, and how do we keep generated builders from
  drifting against the committed IDL?
- Does this need a new upgrade-specific receipt type, or can RFC-0003's body be
  extended with optional `program_id` / `build_hash` fields?
- How does an external party reproduce the build to independently confirm the
  recorded build hash?
