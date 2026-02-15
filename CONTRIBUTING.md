# Contributing to ZVault

Thanks for your interest in contributing. This guide covers everything you need.

## Quick Start

```bash
git clone https://github.com/VanitasCaesar1/zvault.git
cd zvault
cargo build --workspace
cargo test --workspace
cargo run --package zvault-server
```

## Prerequisites

- Rust 2024 edition (install via [rustup](https://rustup.rs))
- Node.js 18+ (for dashboard and website, optional)

## Project Structure

```
crates/
├── zvault-core/       # Barrier, seal, tokens, policies, audit, engines
├── zvault-storage/    # StorageBackend trait + implementations
├── zvault-server/     # HTTP server, routes, middleware, web UI
└── zvault-cli/        # CLI client, MCP server, license system
```

## Code Standards

- `cargo fmt` — non-negotiable
- `cargo clippy --workspace` — pedantic warnings, several lints denied
- No `.unwrap()` or `.expect()` in production code
- No `panic!`, `todo!`, `unimplemented!` in non-test code
- No `unsafe` code
- `thiserror` for library crates, `anyhow` only in binaries
- Newtypes for domain concepts (`SecretPath`, `TokenHash`)
- All key material must implement `Zeroize` and `ZeroizeOnDrop`
- Never log secret values or tokens
- `subtle::ConstantTimeEq` for token comparison

## Workflow

1. Fork the repo
2. Create a feature branch from `main`
3. Make changes, run `cargo fmt` and `cargo clippy`
4. Run `cargo test --workspace`
5. Open a pull request

## Commit Messages

Use conventional commits: `feat:`, `fix:`, `docs:`, `test:`, `chore:`, `security:`

## Testing

```bash
cargo test --workspace                    # All tests
cargo test --package zvault-core         # Specific crate
cargo test --workspace -- --nocapture     # With output
```

Every public function needs at least one test. Crypto functions need known-answer tests.

## Areas Where Help is Wanted

- Storage backends (S3, etcd, Consul)
- Auth methods (OIDC, Kubernetes, AppRole)
- CLI improvements and UX
- Documentation and examples
- Security audits

## License

By contributing, you agree your contributions will be dual-licensed under MIT and Apache 2.0.
