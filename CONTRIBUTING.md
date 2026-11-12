# Contributing

Thanks for your interest in this **unofficial** community project for Alpaca API access in Rust. It is not affiliated with or endorsed by Alpaca.

## Before coding

- Search existing issues and open a **Feature request**, **Bug report**, or **Experiment proposal** if your idea isn't covered.
- Keep changes focused: one issue, one branch, and one pull request where practical.
- Discuss substantial API or architectural changes in an issue before implementing them.

## Build and test

Use this repository's pinned toolchain and run:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

Please add regression tests for bug fixes and tests for new behavior. Document any manual verification in the PR.

## Open a pull request

- Describe the reason for the change, link the relevant issue, and complete the PR template.
- Keep the history readable. We use squash merging where enabled; multiple incremental commits on your branch are fine.
- Address review feedback constructively. Passing automated checks are necessary where CI is configured.

## Privacy and security

Never commit or paste Alpaca API keys, secrets, or real paper/live account payloads. Use local environment variables or ignored configuration files and redact logs before posting them. Do not disclose potential security vulnerabilities in public issues; use [GitHub private vulnerability reporting](SECURITY.md).

## Scope

Use the labels `feature`, `bug`, `experiment`, `refactoring`, and `infrastructure` to help classify work. Issues and PRs should be understandable to someone encountering this repository for the first time.

## Releases

Maintainers: see [RELEASING.md](RELEASING.md) for versioning, checks, tagging, and GitHub Releases.
