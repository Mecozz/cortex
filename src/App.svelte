<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import BackupPanel from "./lib/BackupPanel.svelte";
  import HealthPanel from "./lib/HealthPanel.svelte";
  import MemoryBrowser from "./lib/MemoryBrowser.svelte";
  import MessageList from "./lib/MessageList.svelte";
  import ChatInput from "./lib/ChatInput.svelte";
  import Settings from "./lib/Settings.svelte";
  import TaskPanel from "./lib/TaskPanel.svelte";
  import ToolsPanel from "./lib/ToolsPanel.svelte";
  import VaultPanel from "./lib/VaultPanel.svelte";

  interface Message {
    id: string;
    role: "user" | "assistant";
    content: string;
    provider?: string;
    status?: "queued" | "sending" | "done" | "canceled" | "stopped";
  }

  interface QueueItem {
    id: string;
    text: string;
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
  let processing = $state(false);
  let stopped = $state(false);
  let queue: QueueItem[] = $state([]);
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

  function setStatus(id: string, status: Message["status"]) {
    messages = messages.map((m) => (m.id === id ? { ...m, status } : m));
  }

  // Enqueue a message. Input is never locked — you can keep typing/sending
  // while a reply is generating; extra messages wait in the queue.
  function send() {
    const text = input.trim();
    if (!text) return;
    input = "";
    error = "";
    const id = crypto.randomUUID();
    messages = [
      ...messages,
      { id, role: "user", content: text, status: processing ? "queued" : "sending" },
    ];
    queue = [...queue, { id, text }];
    scrollToBottom();
    drain();
  }

  // Process the queue one item at a time (claude --resume keeps context, so we
  // only send the latest user message per turn).
  async function drain() {
    if (processing) return;
    const item = queue[0];
    if (!item) return;

    processing = true;
    stopped = false;
    setStatus(item.id, "sending");
    loading = true;
    scrollToBottom();

    try {
      // Send the real conversation transcript (excluding queued/canceled items)
      // so the API/Ollama providers have multi-turn context. The claudecode
      // provider only uses the latest user message (it resumes server-side), so
      // this is harmless there and necessary for the stateless providers.
      const transcript = messages
        .filter((m) => m.status !== "queued" && m.status !== "canceled")
        .map(({ role, content }) => ({ role, content }));
      const resp: CompletionResponse = await invoke("chat_message", {
        messages: transcript,
      });
      if (!stopped && resp.content) {
        messages = [
          ...messages,
          {
            id: crypto.randomUUID(),
            role: "assistant",
            content: resp.content,
            provider: resp.provider,
            status: "done",
          },
        ];
        invoke("remember_turn", {
          messages: [
            { role: "user", content: item.text },
            { role: "assistant", content: resp.content },
          ],
          conversationId,
        }).catch(() => {});
      } else if (stopped) {
        setStatus(item.id, "stopped");
      }
    } catch (e) {
      if (stopped) setStatus(item.id, "stopped");
      else error = String(e);
    } finally {
      loading = false;
      processing = false;
      stopped = false;
      queue = queue.filter((q) => q.id !== item.id);
      scrollToBottom();
      if (queue.length) drain();
    }
  }

  // Stop the currently-generating reply (kills the claude process). Queued
  // messages after it still run — cancel them individually if you don't want them.
  function stop() {
    stopped = true;
    invoke("stop_chat").catch(() => {});
  }

  // Remove a still-waiting message from the queue before it runs.
  function cancelQueued(id: string) {
    queue = queue.filter((q) => q.id !== id);
    setStatus(id, "canceled");
  }

  function scrollToBottom() {
    setTimeout(() => {
      if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
    }, 10);
  }

  function clearChat() {
    messages = [];
    queue = [];
    error = "";
    stopped = false;
    invoke("clear_claude_session").catch(() => {});
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
          <MessageList {messages} {loading} onCancel={cancelQueued} />
        </div>

        {#if error}
          <div class="error-bar">{error}</div>
        {/if}

        <ChatInput bind:value={input} {processing} onSend={send} onStop={stop} />
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
    min-height: 0;
    display: flex;
    flex-direction: column;
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

  .error-bar {
    background: #3b0a0a;
    border-top: 1px solid #7f1d1d;
    color: #fca5a5;
    padding: 8px 16px;
    font-size: 13px;
  }

</style>
