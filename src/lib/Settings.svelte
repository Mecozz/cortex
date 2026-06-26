<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import SettingsExtras from "./SettingsExtras.svelte";

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

  let { onClose } = $props<{ onClose: () => void }>();

  let settings: Settings = $state({
    api_key_anthropic: "",
    api_key_openai: "",
    provider: "claudecode",
    model: "claude-opus-4-8",
    system_prompt: "",
    fallback_policy: "hard_fail",
    ollama_url: "http://localhost:11434",
    privacy_mode: false,
    local_only: false,
    sync_folder: "",
    tool_review: "auto",
    claude_access: "chat",
    claude_workdir: "",
  });

  let saving = $state(false);
  let saved = $state(false);
  let loadError = $state("");

  const claudeModels = [
    "claude-fable-5",
    "claude-opus-4-8",
    "claude-sonnet-4-6",
    "claude-haiku-4-5-20251001",
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

  let oauthLoading = $state(false);
  let oauthMsg = $state("");
  const oauthLogin = () => {
    oauthLoading = true;
    oauthMsg = "";
    invoke<string>("oauth_login")
      .then((t) => {
        settings.api_key_anthropic = t;
        oauthMsg = "Connected to Claude.ai. Click Save to keep it.";
      })
      .catch((e) => (oauthMsg = "Login failed: " + String(e)))
      .finally(() => (oauthLoading = false));
  };

</script>

<div class="settings">
  <div class="settings-header">
    <h2>Settings</h2>
    <button class="close-btn" onclick={onClose}>&#x2715;</button>
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
          <option value="claudecode">Claude Code (Max subscription)</option>
          <option value="claude">Claude (API key)</option>
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
          placeholder="sk-ant-api03-... or sk-ant-oat01-... (subscription)"
          autocomplete="off"
        />
      </label>
      <button type="button" onclick={oauthLogin} disabled={oauthLoading}>{oauthLoading ? "Opening browser..." : "Connect Claude.ai"}</button>
      {#if oauthMsg}
        <p class="smsg">{oauthMsg}</p>
      {/if}
      <label>
        Model
        <select bind:value={settings.model}>
          {#if settings.provider === "claude" || settings.provider === "claudecode"}
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
      <h3>Agent Access</h3>
      <label>
        Claude Code access level
        <select bind:value={settings.claude_access}>
          <option value="chat">Chat only &#x2014; read-only, sandboxed (safe default)</option>
          <option value="full">Full access &#x2014; read/edit files + run commands</option>
        </select>
      </label>
      {#if settings.claude_access === "full"}
        <p class="warn">
          &#x26A0; Full access lets Claude read, edit, and delete files and run
          commands in the working directory below. Only enable on a machine you
          control.
        </p>
        <label>
          Working directory (blank = your home folder)
          <input
            type="text"
            bind:value={settings.claude_workdir}
            placeholder="C:\Users\you  (blank = home; use a drive root for whole-PC access)"
          />
        </label>
      {/if}
    </section>

    <section>
      <h3>Ollama</h3>
      <label>
        Base URL
        <input type="text" bind:value={settings.ollama_url} placeholder="http://localhost:11434" />
      </label>
    </section>

    <SettingsExtras bind:settings />
  </div>

  <div class="settings-footer">
    {#if saved}
      <span class="saved-msg">Saved &#x2713;</span>
    {/if}
    <button onclick={save} disabled={saving}>
      {saving ? "Saving..." : "Save"}
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
  select {
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
  select:focus {
    border-color: #5b21b6;
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

  .smsg {
    font-size: 12px;
    color: #aaa;
  }

  .warn {
    font-size: 12px;
    color: #fbbf24;
    background: #2a1f05;
    border: 1px solid #5c4708;
    border-radius: 6px;
    padding: 8px 10px;
    line-height: 1.4;
  }
</style>
