<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface Tool {
    id: string;
    name: string;
    description: string;
    code: string;
    version: string;
    is_active: boolean;
    created_at: number;
  }

  let { onClose } = $props<{ onClose: () => void }>();

  let tools: Tool[] = $state([]);
  let busy = $state(false);
  let err = $state("");
  let msg = $state("");
  let forgeName = $state("");
  let forgeDesc = $state("");
  let forgedCode = $state("");
  let runId = $state("");
  let runResult = $state("");

  onMount(load);

  async function load() {
    try {
      tools = await invoke<Tool[]>("list_tools");
    } catch (e) {
      err = String(e);
    }
  }

  async function del(id: string) {
    busy = true;
    err = "";
    try {
      await invoke("delete_tool", { id });
      await load();
      msg = "Deleted.";
    } catch (e) {
      err = String(e);
    } finally {
      busy = false;
    }
  }

  async function run(id: string) {
    runId = id;
    runResult = "";
    busy = true;
    err = "";
    try {
      runResult = await invoke<string>("run_tool", { id, args: [] });
    } catch (e) {
      err = String(e);
    } finally {
      busy = false;
    }
  }

  async function forge() {
    if (!forgeName.trim() || !forgeDesc.trim()) return;
    busy = true;
    err = "";
    forgedCode = "";
    try {
      forgedCode = await invoke<string>("forge_tool", { description: forgeDesc });
    } catch (e) {
      err = String(e);
    } finally {
      busy = false;
    }
  }

  async function save() {
    busy = true;
    err = "";
    try {
      await invoke("save_tool", {
        tool: {
          id: "",
          name: forgeName.trim(),
          description: forgeDesc.trim(),
          code: forgedCode,
          version: "0.1.0",
          is_active: true,
          created_at: 0,
        },
      });
      forgeName = "";
      forgeDesc = "";
      forgedCode = "";
      await load();
      msg = "Saved.";
    } catch (e) {
      err = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="panel">
  <div class="hdr">
    <h2>Tools</h2>
    <button class="x" onclick={onClose}>&#x2715;</button>
  </div>

  {#if err || msg}
    <p class={err ? "err" : "ok"}>{err || msg}</p>
  {/if}

  <div class="body">
    <section>
      <h3>Installed tools</h3>
      {#if tools.length === 0}
        <p class="empty">No tools yet — forge one below.</p>
      {:else}
        {#each tools as t}
          <div class="tool-row">
            <div class="tool-info">
              <span class="tool-name">{t.name}</span>
              <span class="tool-desc">{t.description}</span>
            </div>
            <div class="tool-btns">
              <button onclick={() => run(t.id)} disabled={busy}>Run</button>
              <button class="danger" onclick={() => del(t.id)} disabled={busy}>Del</button>
            </div>
          </div>
          {#if runId === t.id && runResult}
            <pre class="result">{runResult}</pre>
          {/if}
        {/each}
      {/if}
    </section>

    <section>
      <h3>Forge a new tool</h3>
      <label>
        Name
        <input type="text" bind:value={forgeName} placeholder="e.g. word_count" />
      </label>
      <label>
        Description (what it does + sample input)
        <textarea bind:value={forgeDesc} rows="3" placeholder="Count words in args[0]"></textarea>
      </label>
      <button onclick={forge} disabled={busy || !forgeName.trim() || !forgeDesc.trim()}>
        {busy && !forgedCode ? "Forging..." : "Forge"}
      </button>
      {#if forgedCode}
        <pre class="code">{forgedCode}</pre>
        <div class="code-btns">
          <button onclick={save} disabled={busy}>Save tool</button>
          <button class="ghost" onclick={() => (forgedCode = "")}>Discard</button>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .hdr {
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

  .x {
    background: none;
    border: none;
    color: #666;
    cursor: pointer;
    font-size: 16px;
    padding: 2px 6px;
  }

  .x:hover {
    color: #e8e8ec;
  }

  .err,
  .ok {
    padding: 6px 16px;
    font-size: 13px;
    flex-shrink: 0;
  }

  .err {
    color: #fca5a5;
  }

  .ok {
    color: #4ade80;
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 24px;
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

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: #aaa;
  }

  .empty {
    font-size: 13px;
    color: #444;
  }

  .tool-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    background: #18181f;
    border: 1px solid #2a2a32;
    border-radius: 6px;
    gap: 8px;
  }

  .tool-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .tool-name {
    font-size: 13px;
    color: #e8e8ec;
    font-weight: 500;
  }

  .tool-desc {
    font-size: 11px;
    color: #555;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tool-btns,
  .code-btns {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .result,
  .code {
    font-size: 12px;
    background: #0f0f11;
    border: 1px solid #2a2a32;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
    border-radius: 6px;
    padding: 8px 10px;
  }

  .result {
    color: #a78bfa;
  }

  .code {
    color: #e8e8ec;
    max-height: 200px;
    overflow-y: auto;
  }

  input,
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
    padding: 6px 14px;
  }

  button:hover:not(:disabled) {
    background: #6d28d9;
  }

  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  button.danger {
    background: #7f1d1d;
  }

  button.danger:hover:not(:disabled) {
    background: #991b1b;
  }

  button.ghost {
    background: transparent;
    border: 1px solid #333;
    color: #888;
  }

  button.ghost:hover:not(:disabled) {
    border-color: #555;
    color: #e8e8ec;
  }
</style>
