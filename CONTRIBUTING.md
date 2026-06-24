# Contributing

## Code Rules (auto-enforced by CI)

- Max 400 lines per file, 50 lines per function
- No cross-module imports -- all communication via BUS
- Every module must implement the health contract (health + reset + README)
- 80% test coverage -- tests must run brain-offline (no live DB required)
- No hardcoded secrets or absolute paths
- Conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `test:`

## Pull Requests

1. Fork and create a feature branch from `main`
2. Make your changes following the code rules above
3. Run `cargo test` and `pnpm check` locally before submitting
4. Open a PR with a clear description of what and why

## Registry Tools

Tools submitted to the Cortex registry must:

- Be licensed under Apache 2.0, MIT, or BSD
- Declare all capabilities honestly
- Pass the automated capability scan
- Be published by a verified account
