# Security Policy

Logios is **experimental, devnet-stage software**. It has not been audited
end to end. Run it against value you are willing to lose.

## Supported Versions

Security fixes land on the latest minor release. Older minors are not patched.

| Version | Supported          |
| ------- | ------------------ |
| 0.5.x   | :white_check_mark: |
| 0.4.x   | :white_check_mark: |
| 0.3.x   | :x:                |
| < 0.3   | :x:                |

## Reporting a Vulnerability

Email **security@hermes-labs.xyz** with a description, affected component,
and a reproduction if you have one. Encrypt with our published key if the
report includes a working exploit against consensus or slashing.

Do not open a public issue for an unpatched vulnerability.

We acknowledge within 72 hours and aim to ship a fix or mitigation for
in-scope reports within 14 days. We will credit reporters in the
`CHANGELOG.md` unless you ask otherwise.

## Scope

In scope, in priority order:

- **Consensus** (`crates/logios-consensus`) — leader selection, slot
  finalization, Tower-BFT vote tracking, any path to two valid slots at one
  height.
- **Slashing & jail** (`crates/logios-consensus`) — false-positive slashing,
  evasion of a provable equivocation, jail-escape.
- **Runtime** (`crates/logios-runtime`) — compute-budget bypass, account
  ownership violations, executor state corruption.
- **Programs** (`programs/receipt-registry`, `programs/hermes-faucet`) —
  missing signer/owner checks, receipt forgery, faucet drain.

## Out of Scope

- The website, docs, and `/v1` read API rate-limiting (report via normal
  issues).
- Findings that require a compromised leader key — the autonomous leader key
  is a trusted component on devnet.
- $HERMES market behavior. The token carries no value and no guarantees.

## Disclosure

We practice coordinated disclosure. Once a fix is released we publish an
advisory in the repository's Security tab with the affected versions and a
credit line.
