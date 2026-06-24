<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import BackupPanel from "./lib/BackupPanel.svelte";
  import HealthPanel from "./lib/HealthPanel.svelte";
  import MemoryBrowser from "./lib/MemoryBrowser.svelte";
  import Settings from "./lib/Settings.svelte";
  import TaskPanel from "./lib/TaskPanel.svelte";
  import ToolsPanel from "./lib/ToolsPanel.svelte";
  import VaultPanel from "./lib/VaultPanel.svelte";

  interface Message {
    role: "user" | "assistant";
    content: string;
    provider?: string;
  }

  interface CompletionResponse {
    content: string;
    provider: string;
  }

  interface BrainStatus {
    overall: string;
    modules: unknown[];
  }

  let messages: Message[] = $state([]);
  let input = $state("");
  let loading = $state(false);
  let error = $state("");
  let activePanel = $state("");
  let messagesEl: HTMLElement | undefined = $state();
  let brainStatus = $state("green");

  const conversationId = crypto.randomUUID();

  const statusColor: Record<string, string> = {
    green: "#4ade80",
    yellow: "#fbbf24",
    red: "#f87171",
  };

  const go = (p: string) => () => (activePanel = p);

  async function refreshStatus() {
    const s = await invoke<BrainStatus>("get_brain_status").catch(() => null);
    if (s) brainStatus = s.overall;
  }

  refreshStatus();
  setInterval(refreshStatus, 30_000);

  async function send() {
    const text = input.trim();
    if (!text || loading) return;

    input = "";
    error = "";
    messages = [...messages, { role: "user", content: text }];
    loading = true;
    scrollToBottom();

    try {
      const wire = messages.map(({ role, content }) => ({ role, content }));
      const resp: CompletionResponse = await invoke("chat_message", { messages: wire });
      if (resp.content) {
        messages = [
          ...messages,
          { role: "assistant", content: resp.content, provider: resp.provider },
        ];
        invoke("remember_turn", {
          messages: messages.slice(-6).map(({ role, content }) => ({ role, content })),
          conversationId,
        }).catch(() => {});
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
      scrollToBottom();
    }
  }

  function scrollToBottom() {
    setTimeout(() => {
      if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
    }, 10);
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  function clearChat() {
    messages = [];
    error = "";
  }
</script>

<div class="app">
  <header>
    <span class="logo">&#x2728; Cortex</span>
    <div class="header-actions">
      <button
        class="status-dot"
        onclick={go("health")}
        title="Brain health"
        style="color: {statusColor[brainStatus] ?? '#888'}">&#x25CF;</button
      >
      <button class="i" onclick={clearChat} title="Clear chat">🗑</button>
      <button class="i" onclick={go("memory")} title="Memory">🧠</button>
      <button class="i" onclick={go("tasks")} title="Tasks">✓</button>
      <button class="i" onclick={go("tools")} title="Tools">🔧</button>
      <button class="i" onclick={go("vault")} title="Vault">🔑</button>
      <button class="i" onclick={go("backup")} title="Backup">💾</button>
      <button class="i" onclick={go("settings")} title="Settings">⚙</button>
    </div>
  </header>

  <div class="body">
    {#if activePanel === "settings"}
      <Settings onClose={go("")} />
    {:else if activePanel === "memory"}
      <MemoryBrowser onClose={go("")} />
    {:else if activePanel === "tasks"}
      <TaskPanel onClose={go("")} />
    {:else if activePanel === "health"}
      <HealthPanel onClose={go("")} />
    {:else if activePanel === "vault"}
      <VaultPanel onClose={go("")} />
    {:else if activePanel === "backup"}
      <BackupPanel onClose={go("")} />
    {:else if activePanel === "tools"}
      <ToolsPanel onClose={go("")} />
    {:else}
      <main>
        <div class="messages" bind:this={messagesEl}>
          {#if messages.length === 0}
            <div class="empty">Start a conversation</div>
          {/if}
          {#each messages as msg}
            <div class="message {msg.role}">
              <div class="bubble">{msg.content}</div>
              {#if msg.role === "assistant" && msg.provider}
                <span class="provider-tag">{msg.provider}</span>
              {/if}
            </div>
          {/each}
          {#if loading}
            <div class="message assistant">
              <div class="bubble thinking">...</div>
            </div>
          {/if}
        </div>

        {#if error}
          <div class="error-bar">{error}</div>
        {/if}

        <div class="input-row">
          <textarea
            bind:value={input}
            onkeydown={handleKey}
            placeholder="Message... (Enter to send, Shift+Enter for newline)"
            rows="3"
            disabled={loading}
          ></textarea>
          <button onclick={send} disabled={loading || !input.trim()}>Send</button>
        </div>
      </main>
    {/if}
  </div>
</div>

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    font-family:
      system-ui,
      -apple-system,
      sans-serif;
    background: #0f0f11;
    color: #e8e8ec;
    height: 100vh;
    overflow: hidden;
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid #222228;
    flex-shrink: 0;
  }

  .logo {
    font-size: 16px;
    font-weight: 600;
    color: #a78bfa;
  }

  .header-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .status-dot {
    background: none;
    border: none;
    font-size: 16px;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
    transition: opacity 0.15s;
  }

  .status-dot:hover {
    opacity: 0.7;
  }

  .i {
    background: none;
    border: 1px solid #333;
    color: #888;
    cursor: pointer;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 14px;
    transition:
      color 0.15s,
      border-color 0.15s;
  }

  .i:hover {
    color: #e8e8ec;
    border-color: #555;
  }

  .body {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .empty {
    color: #444;
    text-align: center;
    margin-top: 40px;
    font-size: 14px;
  }

  .message {
    display: flex;
    flex-direction: column;
  }

  .message.user {
    align-items: flex-end;
  }

  .bubble {
    max-width: 72%;
    padding: 10px 14px;
    border-radius: 12px;
    font-size: 14px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .message.user .bubble {
    background: #5b21b6;
    color: #fff;
    border-bottom-right-radius: 4px;
  }

  .message.assistant .bubble {
    background: #1c1c22;
    border: 1px solid #2a2a32;
    border-bottom-left-radius: 4px;
  }

  .provider-tag {
    font-size: 10px;
    color: #444;
    margin-top: 3px;
    margin-left: 2px;
  }

  .thinking {
    color: #555;
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.4;
    }
    50% {
      opacity: 1;
    }
  }

  .error-bar {
    background: #3b0a0a;
    border-top: 1px solid #7f1d1d;
    color: #fca5a5;
    padding: 8px 16px;
    font-size: 13px;
  }

  .input-row {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid #222228;
    flex-shrink: 0;
  }

  textarea {
    flex: 1;
    background: #18181f;
    border: 1px solid #333;
    border-radius: 8px;
    color: #e8e8ec;
    padding: 8px 12px;
    font-family: inherit;
    font-size: 14px;
    resize: none;
    outline: none;
    transition: border-color 0.15s;
  }

  textarea:focus {
    border-color: #5b21b6;
  }

  textarea:disabled {
    opacity: 0.5;
  }

  button[onclick] {
    background: #5b21b6;
    border: none;
    border-radius: 8px;
    color: #fff;
    cursor: pointer;
    font-size: 14px;
    padding: 0 20px;
    transition: background 0.15s;
  }

  button[onclick]:hover:not(:disabled) {
    background: #6d28d9;
  }

  button[onclick]:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
