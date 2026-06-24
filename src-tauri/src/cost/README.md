# cost

Usage tracker. Logs every API call to the `usage` SQLite table with token counts and
estimated USD cost.

## Cost rates

Rates in `mod.rs` are hardcoded approximations. Update `cost_per_mtok()` when provider
pricing changes. Not auto-fetched.

## Phase 1 scope

- `log_usage()` — insert a `UsageEntry` row.
- `estimate_cost()` — calculate approximate USD from token counts.
