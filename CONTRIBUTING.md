# Contributing to Logios

Thanks for looking. Logios is built in public and reviewed by humans even
though slots are authored by an agent. The bar is the same as any serious
Solana protocol: spec-first, tested, and signed off.

## Before you start

For anything beyond a typo, open an issue or comment on an existing one so we
can agree on the approach. Substantial design changes go through an RFC in
[`spec/`](spec/) before code lands. Read the RFC index first — your idea may
already be tracked.

## Branch strategy

- `main` — the released line. Protected. Only `develop` merges in, at a tag.
- `develop` — integration branch. PRs target this.
- `feat/<short-name>` — feature branches off `develop`.
- `fix/<short-name>` — bug fixes off `develop` (or `main` for hotfixes).

Rebase your branch on the latest `develop` before opening a PR. Keep history
linear; we squash-merge unless a series of commits is genuinely independent.

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org/).

```
feat(runtime): meter syscalls against the per-slot compute budget
fix(consensus): reject a second vote at an already-voted slot
docs(receipts): clarify base58 signature encoding
chore(ci): cache cargo registry between jobs
```

Scopes track the workspace layout: `primitives`, `runtime`, `consensus`,
`ledger`, `receipt-registry`, `hermes-faucet`, `sdk-ts`, `sdk-rust`, `cli`,
`spec`, `docs`, `ci`.

## Local checks

Run these before pushing. CI runs the same set.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
anchor build          # requires the Solana + Anchor toolchain
cd sdk/ts && npm ci && npm test
```

New behavior needs a test. Consensus and slashing changes need a test that
fails without the change. The `#![forbid(unsafe_code)]` lint stays on in the
core crates.

## Pull requests

- Fill in the PR template, including which RFC the change implements or amends.
- A PR needs at least one approving review from a code owner of every path it
  touches (see [`.github/CODEOWNERS`](.github/CODEOWNERS)).
- Keep PRs focused. A diff that touches consensus and the SDK and the docs is
  three reviews waiting to time out — split it.
- Green CI is required to merge.

## Maintainers

| Area | Owner |
| --- | --- |
| Runtime + consensus, lead | **hermeslabssol** |
| Runtime / SVM executor | **Niko Vasilakis** |
| Consensus / Anchor programs | **Aris Lefebvre** |
| SDK / developer experience | **Demetra Soto** |

## License

By contributing you agree your work is licensed under Apache-2.0, the same as
the rest of the project.
