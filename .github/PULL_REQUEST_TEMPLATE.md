# Pull request

## Summary

<!-- What does this change and why? One or two sentences. -->

## RFC

<!-- Which RFC does this implement or amend? Link it. If none is needed
     (typo, chore), say so and why. -->

- Implements / amends: <!-- spec/RFC-000X-...md -->

## Type of change

- [ ] `feat` — new behavior
- [ ] `fix` — bug fix
- [ ] `docs` — documentation only
- [ ] `chore` / `ci` — tooling, no runtime change
- [ ] Breaking change (pre-1.0 minor bump)

## Checklist

- [ ] Conventional commit messages, scoped to the workspace layout.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --all-targets --all-features` is clean.
- [ ] `cargo test --all` passes; new behavior has a test.
- [ ] `anchor build` passes (if programs changed).
- [ ] `sdk/ts` builds and tests pass (if the SDK changed).
- [ ] Receipt or consensus changes keep sealing deterministic.
- [ ] Reviewed by a code owner of every touched path.

## Notes for reviewers

<!-- Anything non-obvious: trade-offs, follow-ups, areas you want eyes on. -->
