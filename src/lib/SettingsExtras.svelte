<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";

  interface Settings {
    api_key_anthropic: string;
    api_key_openai: string;
    provider: string;
    model: string;
    system_prompt: string;
    fallback_policy: string;
    ollama_url: string;
    privacy_mode: boolean;
    local_only: boolean;
    sync_folder: string;
    tool_review: string;
    claude_access: string;
    claude_workdir: string;
  }

  let { settings = $bindable() } = $props<{ settings: Settings }>();

  let syncing = $state(false);
  let syncMsg = $state("");
  let checking = $state(false);
  let updateMsg = $state("");
  let importPath = $state("");
  let importing = $state(false);
  let importMsg = $state("");

  function exportSync() {
    syncing = true;
    syncMsg = "";
    invoke("sync_export", { syncFolder: settings.sync_folder })
      .then(() => (syncMsg = "Exported."))
      .catch((e) => (syncMsg = String(e)))
      .finally(() => (syncing = false));
  }

  function importSync() {
    syncing = true;
    syncMsg = "";
    invoke("sync_import", { syncFolder: settings.sync_folder })
      .then(() => (syncMsg = "Import queued — restart to apply."))
      .catch((e) => (syncMsg = String(e)))
      .finally(() => (syncing = false));
  }

  function checkUpdate() {
    checking = true;
    updateMsg = "";
    invoke<string>("check_update")
      .then((v) => (updateMsg = v || "Already up to date."))
      .catch((e) => (updateMsg = String(e)))
      .finally(() => (checking = false));
  }

  async function browse() {
    const f = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
    if (typeof f === "string") importPath = f;
  }

  function importMemories() {
    importing = true;
    importMsg = "";
    invoke<number>("import_memories", { path: importPath })
      .then((n) => (importMsg = `Imported ${n} memories into Cortex.`))
      .catch((e) => (importMsg = String(e)))
      .finally(() => (importing = false));
  }

  function importFull() {
    importing = true;
    importMsg = "Importing (large files take a moment)...";
    invoke<number>("import_data", { path: importPath })
      .then((n) => (importMsg = `Imported ${n} records (facts + messages + relationships).`))
      .catch((e) => (importMsg = String(e)))
      .finally(() => (importing = false));
  }
</script>

<section>
  <h3>Persona / System prompt</h3>
  <textarea
    bind:value={settings.system_prompt}
    placeholder="Optional system prompt shown before every conversation..."
    rows="4"
  ></textarea>
</section>

<section>
  <h3>Privacy</h3>
  <label class="toggle-label">
    <input type="checkbox" bind:checked={settings.privacy_mode} />
    <span>Privacy mode &mdash; disable memory capture</span>
  </label>
  <label class="toggle-label">
    <input type="checkbox" bind:checked={settings.local_only} />
    <span>Local only &mdash; block all cloud providers</span>
  </label>
</section>

<section>
  <h3>Tools</h3>
  <label>
    Tool review mode
    <select bind:value={settings.tool_review}>
      <option value="auto">Auto &#x2014; run without confirmation</option>
      <option value="summary">Summary &#x2014; show name before running</option>
      <option value="full">Full code &#x2014; show code before running</option>
    </select>
  </label>
</section>

<section>
  <h3>Sync</h3>
  <label>
    Sync folder path
    <input type="text" bind:value={settings.sync_folder} placeholder="/path/to/shared/folder" />
  </label>
  <div class="btn-row">
    <button onclick={exportSync} disabled={syncing || !settings.sync_folder}> Export </button>
    <button onclick={importSync} disabled={syncing || !settings.sync_folder}> Import </button>
  </div>
  {#if syncMsg}
    <p class="smsg">{syncMsg}</p>
  {/if}
</section>

<section>
  <h3>Import Memories</h3>
  <p class="hint">
    Load memories from a JSON file into Cortex's brain. The file is an array of
    <code>{`{type, title, content, tags, source, created_at}`}</code> objects
    (e.g. exported from another assistant or brain). Each becomes a recallable fact.
  </p>
  <label>
    JSON file path
    <div class="path-row">
      <input type="text" bind:value={importPath} placeholder="C:\path\to\memories.json" />
      <button class="browse" onclick={browse}>Browse…</button>
    </div>
  </label>
  <div class="btn-row">
    <button onclick={importMemories} disabled={importing || !importPath}>
      {importing ? "Importing..." : "Import memories (simple)"}
    </button>
    <button onclick={importFull} disabled={importing || !importPath}>
      {importing ? "Importing..." : "Import full export"}
    </button>
  </div>
  <p class="hint">
    <b>Simple</b> = a memories array (→ facts). <b>Full export</b> = a native
    Cortex export with facts + messages + relationships.
  </p>
  {#if importMsg}
    <p class="smsg">{importMsg}</p>
  {/if}
</section>

<section>
  <h3>Updates</h3>
  <button onclick={checkUpdate} disabled={checking}> Check for updates </button>
  {#if updateMsg}
    <p class="smsg">{updateMsg}</p>
  {/if}
</section>

<style>
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

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: #aaa;
  }

  .toggle-label {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .toggle-label input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: #5b21b6;
    cursor: pointer;
  }

  input,
  select,
  textarea {
    background: #18181f;
    border: 1px solid #333;
    border-radius: 6px;
    color: #e8e8ec;
    padding: 7px 10px;
    font-family: inherit;
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s;
  }

  input:focus,
  select:focus,
  textarea:focus {
    border-color: #5b21b6;
  }

  textarea {
    resize: vertical;
  }

  button {
    background: #5b21b6;
    border: none;
    border-radius: 6px;
    color: #fff;
    cursor: pointer;
    font-size: 13px;
    padding: 8px 20px;
  }

  button:hover:not(:disabled) {
    background: #6d28d9;
  }

  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-row {
    display: flex;
    gap: 8px;
  }

  .smsg {
    font-size: 12px;
    color: #aaa;
  }

  .hint {
    font-size: 12px;
    color: #888;
    line-height: 1.5;
  }

  .hint code {
    background: #18181f;
    border: 1px solid #333;
    border-radius: 4px;
    padding: 1px 4px;
    font-size: 11px;
  }

  .path-row {
    display: flex;
    gap: 8px;
  }

  .path-row input {
    flex: 1;
  }

  button.browse {
    background: #2a2a32;
    white-space: nowrap;
  }

  button.browse:hover:not(:disabled) {
    background: #3a3a44;
  }
</style>
