# backup

Backup and reset for Cortex.

Backups are created via SQLite `VACUUM INTO` — a consistent, clean snapshot of the brain database.
Backup files live in `{app_data}/backups/`. Oldest are pruned after 30 total.

**Reset levels:**

- L1 Rollback — restore most recent backup (requires app restart)
- L2 Session — clear last 24h of episodic messages
- L3 Facts — wipe all facts and scores buffer
- L4 Tasks — wipe all tasks
- L5 Memory — wipe facts + episodic + scores + tasks
- L6 Brain — wipe all data, keep settings + vault
- L7 Hard — wipe everything including settings

All reset levels L2-L7 auto-create a backup before destroying data.
