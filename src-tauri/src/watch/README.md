# watch

Watchdog and circuit breaker for Cortex.

**WATCH** — aggregates health status from all modules via `get_brain_status` command.  
**CB** — circuit breaker: after 3 consecutive failures, a provider/module is disabled
until the next successful call resets the counter.

Brain status (green/yellow/red) is displayed in the header and updates every 30 seconds.
