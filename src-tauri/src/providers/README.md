# providers

LLM provider adapters. Each adapter implements the `Provider` trait.

## Adapters

- `cloud.rs` — Claude (Anthropic). Primary cloud provider.
- `local.rs` — Ollama. Local model runner.
- `fallback.rs` — `FallbackPolicy` enum: what to do when the active provider fails.
- `health.rs` — `HealthCheck` impl reporting which providers are configured.

## Adding a new provider

1. Create `<name>.rs` implementing the `Provider` trait.
2. Add it to `mod.rs`.
3. Wire it into `commands.rs` provider dispatch.
