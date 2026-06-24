<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface VaultEntry {
    key: string;
    description: string;
  }

  let { onClose } = $props<{ onClose: () => void }>();

  let entries: VaultEntry[] = $state([]);
  let newKey = $state("");
  let newValue = $state("");
  let newDesc = $state("");
  let adding = $state(false);
  let loadError = $state("");

  async function load() {
    try {
      entries = await invoke<VaultEntry[]>("get_vault_keys");
    } catch (e) {
      loadError = String(e);
    }
  }

  onMount(load);

  async function addEntry() {
    if (!newKey.trim() || !newValue.trim()) return;
    adding = true;
    try {
      await invoke("set_vault_item", {
        key: newKey.trim(),
        value: newValue.trim(),
        description: newDesc.trim(),
      });
      newKey = "";
      newValue = "";
      newDesc = "";
      await load();
    } catch (e) {
      loadError = String(e);
    } finally {
      adding = false;
    }
  }

  async function remove(key: string) {
    try {
      await invoke("delete_vault_item", { key });
      await load();
    } catch (e) {
      loadError = String(e);
    }
  }
</script>

<div class="panel">
  <div class="panel-header">
    <h2>Vault</h2>
    <button class="close-btn" onclick={onClose}>&#x2715;</button>
  </div>

  {#if loadError}
    <p class="error">{loadError}</p>
  {/if}

  <div class="panel-body">
    <section>
      <h3>Stored secrets</h3>
      {#if entries.length === 0}
        <p class="empty">No vault entries yet.</p>
      {:else}
        <ul class="entry-list">
          {#each entries as e}
            <li>
              <div class="entry-info">
                <span class="entry-key">{e.key}</span>
                {#if e.description}
                  <span class="entry-desc">{e.description}</span>
                {/if}
              </div>
              <button class="del-btn" onclick={() => remove(e.key)}>Delete</button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section>
      <h3>Add secret</h3>
      <label>
        Key
        <input type="text" bind:value={newKey} placeholder="e.g. api_key_anthropic" />
      </label>
      <label>
        Value
        <input
          type="password"
          bind:value={newValue}
          placeholder="Secret value"
          autocomplete="off"
        />
      </label>
      <label>
        Description (optional)
        <input type="text" bind:value={newDesc} placeholder="What this key is for" />
      </label>
      <button
        class="add-btn"
        onclick={addEntry}
        disabled={adding || !newKey.trim() || !newValue.trim()}
      >
        {adding ? "Saving..." : "Add"}
      </button>
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

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: #aaa;
  }

  input {
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

  input:focus {
    border-color: #5b21b6;
  }

  .empty {
    color: #444;
    font-size: 13px;
  }

  .error {
    padding: 8px 16px;
    color: #fca5a5;
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
  }

  .entry-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .entry-key {
    font-size: 13px;
    color: #e8e8ec;
    font-family: monospace;
  }

  .entry-desc {
    font-size: 11px;
    color: #555;
  }

  .del-btn {
    background: #3b0a0a;
    border: 1px solid #7f1d1d;
    border-radius: 4px;
    color: #fca5a5;
    cursor: pointer;
    font-size: 12px;
    padding: 3px 8px;
  }

  .del-btn:hover {
    background: #5a1111;
  }

  .add-btn {
    background: #5b21b6;
    border: none;
    border-radius: 6px;
    color: #fff;
    cursor: pointer;
    font-size: 13px;
    padding: 8px 20px;
    align-self: flex-start;
  }

  .add-btn:hover:not(:disabled) {
    background: #6d28d9;
  }

  .add-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
