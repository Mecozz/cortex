# sync

Folder-based sync for Cortex.

**SFOLDER**: Exports the brain database to a user-configured folder (Dropbox, iCloud, OneDrive,
or any shared path). Uses `VACUUM INTO` for a clean, consistent snapshot.

**SYNCCONF**: If the sync folder has a newer `cortex.db`, it can be imported via the Settings
panel. The import is queued via `restore_pending.txt` and applied on the next app startup.

**AUTOUPDATE**: Checks GitHub releases for a newer version on demand. No auto-install —
just displays the available version in Settings.
