<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface Settings {
    api_key_anthropic: string;
    api_key_openai: string;
    provider: string;
    model: string;
    system_prompt: string;
    fallback_policy: string;
    ollama_url: string;
  }

  let { onClose } = $props<{ onClose: () => void }>();

  let settings: Settings = $state({
    api_key_anthropic: "",
    api_key_openai: "",
    provider: "claude",
    model: "claude-sonnet-4-6",
    system_prompt: "",
    fallback_policy: "hard_fail",
    ollama_url: "http://localhost:11434",
  });

  let saving = $state(false);
  let saved = $state(false);
  let loadError = $state("");

  const claudeModels = [
    "claude-opus-4-7",
    "claude-sonnet-4-6",
    "claude-haiku-4-5-20251001",
    "claude-3-5-sonnet-20241022",
    "claude-3-haiku-20240307",
  ];

  const ollamaModels = ["llama3.2", "llama3.1", "mistral", "phi3", "gemma2"];

  onMount(async () => {
    try {
      settings = await invoke<Settings>("get_settings");
    } catch (e) {
      loadError = String(e);
    }
  });

  async function save() {
    saving = true;
    saved = false;
    try {
      await invoke("save_settings", { settings });
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } finally {
      saving = false;
    }
  }
</script>

<div class="settings">
  <div class="settings-header">
    <h2>Settings</h2>
    <button class="close-btn" onclick={onClose}>✕</button>
  </div>

  {#if loadError}
    <p class="error">{loadError}</p>
  {/if}

  <div class="settings-body">
    <section>
      <h3>Provider</h3>
      <label>
        Active provider
        <select bind:value={settings.provider}>
          <option value="claude">Claude (Anthropic)</option>
          <option value="ollama">Ollama (local)</option>
        </select>
      </label>
      <label>
        Fallback policy
        <select bind:value={settings.fallback_policy}>
          <option value="hard_fail">Hard fail (show error)</option>
          <option value="transparent">Transparent (banner)</option>
          <option value="silent">Silent (empty response)</option>
        </select>
      </label>
    </section>

    <section>
      <h3>Claude</h3>
      <label>
        API key
        <input
          type="password"
          bind:value={settings.api_key_anthropic}
          placeholder="sk-ant-…"
          autocomplete="off"
        />
      </label>
      <label>
        Model
        <select bind:value={settings.model}>
          {#if settings.provider === "claude"}
            {#each claudeModels as m}
              <option value={m}>{m}</option>
            {/each}
          {:else}
            {#each ollamaModels as m}
              <option value={m}>{m}</option>
            {/each}
          {/if}
        </select>
      </label>
    </section>

    <section>
      <h3>Ollama</h3>
      <label>
        Base URL
        <input type="text" bind:value={settings.ollama_url} placeholder="http://localhost:11434" />
      </label>
    </section>

    <section>
      <h3>Persona / System prompt</h3>
      <textarea
        bind:value={settings.system_prompt}
        placeholder="Optional system prompt shown before every conversation…"
        rows="4"
      ></textarea>
    </section>
  </div>

  <div class="settings-footer">
    {#if saved}
      <span class="saved-msg">Saved ✓</span>
    {/if}
    <button onclick={save} disabled={saving}>
      {saving ? "Saving…" : "Save"}
    </button>
  </div>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .settings-header {
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

  .settings-body {
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

  .settings-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid #222228;
    flex-shrink: 0;
  }

  .saved-msg {
    font-size: 13px;
    color: #4ade80;
  }

  .error {
    padding: 8px 16px;
    color: #fca5a5;
    font-size: 13px;
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
</style>
