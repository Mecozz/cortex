# librarian

Nightly REM cycle for Cortex.

**LIB** — runs every hour in a background Tokio task. Extracts new facts from recent episodic
entries, decays expired SCORES, and consolidates memory.

OS-level scheduling (launchd/systemd/Task Scheduler) is a future OSSVC enhancement;
currently runs in-process while the app is open.
