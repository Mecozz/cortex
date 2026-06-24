<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface BackupEntry {
    name: string;
    filename: string;
    size_bytes: number;
    created_at: number;
  }

  let { onClose } = $props<{ onClose: () => void }>();

  let backups: BackupEntry[] = $state([]);
  let tab = $state<"backups" | "reset" | "data">("backups");
  let newName = $state("");
  let busy = $state(false);
  let confirmLevel = $state(0);
  let statusMsg = $state("");
  let loadError = $state("");
  let portMsg = $state("");

  const resetLevels = [
    { level: 1, label: "L1 Rollback", desc: "Restore most recent backup (restart required)" },
    { level: 2, label: "L2 Session", desc: "Clear last 24h of messages" },
    { level: 3, label: "L3 Facts", desc: "Wipe all extracted facts and scores" },
    { level: 4, label: "L4 Tasks", desc: "Wipe all open tasks" },
    { level: 5, label: "L5 Memory", desc: "Wipe facts + messages + scores + tasks" },
    { level: 6, label: "L6 Brain", desc: "Wipe all data, keep settings and vault" },
    { level: 7, label: "L7 Hard", desc: "Wipe everything including settings" },
  ];

  async function load() {
    try {
      backups = await invoke<BackupEntry[]>("list_backups");
    } catch (e) {
      loadError = String(e);
    }
  }

  onMount(load);

  async function create() {
    busy = true;
    statusMsg = "";
    loadError = "";
    try {
      const name = await invoke<string>("create_backup", { name: newName.trim() || null });
      newName = "";
      statusMsg = `Created: ${name}`;
      await load();
    } catch (e) {
      loadError = String(e);
    } finally {
      busy = false;
    }
  }

  async function restore(filename: string) {
    loadError = "";
    try {
      await invoke("restore_backup", { filename });
      statusMsg = "Restore queued — restart the app to complete.";
    } catch (e) {
      loadError = String(e);
    }
  }

  async function del(filename: string) {
    loadError = "";
    try {
      await invoke("delete_backup", { filename });
      await load();
    } catch (e) {
      loadError = String(e);
    }
  }

  async function doReset(level: number) {
    if (confirmLevel !== level) {
      confirmLevel = level;
      return;
    }
    confirmLevel = 0;
    busy = true;
    statusMsg = "";
    loadError = "";
    try {
      const result = await invoke<string>("reset_level", { level });
      statusMsg =
        result === "restart_required"
          ? "Rollback queued — restart to complete."
          : `Reset level ${level} complete.`;
    } catch (e) {
      loadError = String(e);
    } finally {
      busy = false;
    }
  }

  const fmt = (ts: number) => new Date(ts * 1000).toLocaleString();
  const fmtSize = (b: number) =>
    b < 1048576 ? `${(b / 1024).toFixed(1)} KB` : `${(b / 1048576).toFixed(1)} MB`;
  const exportJson = () =>
    invoke<string>("export_data")
      .then((p) => (portMsg = p))
      .catch((e) => (portMsg = String(e)));
</script>

<div class="panel">
  <div class="panel-header">
    <h2>Backup &amp; Reset</h2>
    <button class="close-btn" onclick={onClose}>&#x2715;</button>
  </div>

  <div class="tabs">
    <button class:active={tab === "backups"} onclick={() => (tab = "backups")}>Backups</button>
    <button class:active={tab === "data"} onclick={() => (tab = "data")}>Data</button>
    <button class:active={tab === "reset"} onclick={() => (tab = "reset")}>Reset</button>
  </div>

  {#if loadError}
    <p class="msg err">{loadError}</p>
  {/if}
  {#if statusMsg}
    <p class="msg ok">{statusMsg}</p>
  {/if}

  <div class="panel-body">
    {#if tab === "backups"}
      <section>
        <h3>Create backup</h3>
        <div class="row">
          <input type="text" bind:value={newName} placeholder="Optional name (blank = timestamp)" />
          <button class="action-btn" onclick={create} disabled={busy}>
            {busy ? "..." : "Create"}
          </button>
        </div>
      </section>

      <section>
        <h3>Saved backups</h3>
        {#if backups.length === 0}
          <p class="empty">No backups yet.</p>
        {:else}
          <ul class="entry-list">
            {#each backups as b}
              <li>
                <div class="entry-info">
                  <span class="entry-name">{b.name}</span>
                  <span class="entry-meta"
                    >{fmtSize(b.size_bytes)} &middot; {fmt(b.created_at)}</span
                  >
                </div>
                <div class="row">
                  <button class="sm-btn" onclick={() => restore(b.filename)}>Restore</button>
                  <button class="sm-btn danger" onclick={() => del(b.filename)}>Delete</button>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {:else}
      <section>
        <h3>Reset levels</h3>
        <p class="empty">
          L2&ndash;L7 auto-backup before wiping. Click once to arm, again to confirm.
        </p>
        <ul class="entry-list">
          {#each resetLevels as r}
            <li>
              <div class="entry-info">
                <span class="entry-name">{r.label}</span>
                <span class="entry-meta">{r.desc}</span>
              </div>
              <button
                class="sm-btn danger {confirmLevel === r.level ? 'armed' : ''}"
                onclick={() => doReset(r.level)}
                disabled={busy}
              >
                {confirmLevel === r.level ? "Confirm" : "Reset"}
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-bottom: 1px solid #222228;
    flex-shrink: 0;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
    color: #e8e8ec;
  }

  .close-btn {
    background: none;
    border: none;
    color: #666;
    cursor: pointer;
    font-size: 16px;
    padding: 2px 6px;
  }

  .close-btn:hover {
    color: #e8e8ec;
  }

  .tabs {
    display: flex;
    border-bottom: 1px solid #222228;
    flex-shrink: 0;
  }

  .tabs button {
    background: none;
    border: none;
    color: #555;
    cursor: pointer;
    font-size: 13px;
    padding: 8px 16px;
    border-bottom: 2px solid transparent;
  }

  .tabs button.active {
    color: #e8e8ec;
    border-bottom-color: #5b21b6;
  }

  .msg {
    padding: 8px 16px;
    font-size: 13px;
    flex-shrink: 0;
  }

  .msg.err {
    color: #fca5a5;
  }

  .msg.ok {
    color: #4ade80;
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #555;
    font-weight: 600;
  }

  .row {
    display: flex;
    gap: 6px;
  }

  input {
    flex: 1;
    background: #18181f;
    border: 1px solid #333;
    border-radius: 6px;
    color: #e8e8ec;
    padding: 7px 10px;
    font-family: inherit;
    font-size: 13px;
    outline: none;
  }

  input:focus {
    border-color: #5b21b6;
  }

  .empty {
    color: #444;
    font-size: 13px;
  }

  .entry-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .entry-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: #18181f;
    border: 1px solid #2a2a32;
    border-radius: 6px;
    padding: 8px 12px;
    gap: 8px;
  }

  .entry-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .entry-name {
    font-size: 13px;
    color: #e8e8ec;
    font-family: monospace;
  }

  .entry-meta {
    font-size: 11px;
    color: #555;
  }

  .action-btn {
    background: #5b21b6;
    border: none;
    border-radius: 6px;
    color: #fff;
    cursor: pointer;
    font-size: 13px;
    padding: 7px 14px;
    flex-shrink: 0;
  }

  .action-btn:hover:not(:disabled) {
    background: #6d28d9;
  }

  .action-btn:disabled,
  .sm-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .sm-btn {
    background: #1c1c22;
    border: 1px solid #333;
    border-radius: 4px;
    color: #aaa;
    cursor: pointer;
    font-size: 12px;
    padding: 3px 8px;
    flex-shrink: 0;
  }

  .sm-btn:hover:not(:disabled) {
    color: #e8e8ec;
    border-color: #555;
  }

  .sm-btn.danger {
    color: #fca5a5;
    border-color: #7f1d1d;
  }

  .sm-btn.armed {
    background: #450a0a;
  }
</style>
