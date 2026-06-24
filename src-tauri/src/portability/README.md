# portability

Data portability module for Cortex (EXPORT/IMPORT).

**export_json**: Serializes all brain tables (proj, facts, episodic, tasks, convo, rel, tools) to `cortex_export.json` in the app data directory. Returns the file path.

**import_json**: Reads a Cortex JSON export file and inserts rows into the corresponding tables using `INSERT OR IGNORE` (no duplicates, no overwrites).
