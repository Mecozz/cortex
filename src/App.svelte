<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Settings from "./lib/Settings.svelte";

  interface Message {
    role: "user" | "assistant";
    content: string;
  }

  interface CompletionResponse {
    content: string;
    input_tokens: number;
    output_tokens: number;
    model: string;
    provider: string;
  }

  let messages: Message[] = $state([]);
  let input = $state("");
  let loading = $state(false);
  let error = $state("");
  let showSettings = $state(false);
  let messagesEl: HTMLElement | undefined = $state();

  async function send() {
    const text = input.trim();
    if (!text || loading) return;

    input = "";
    error = "";
    messages = [...messages, { role: "user", content: text }];
    loading = true;
    scrollToBottom();

    try {
      const resp: CompletionResponse = await invoke("chat_message", {
        messages: messages,
      });
      if (resp.content) {
        messages = [...messages, { role: "assistant", content: resp.content }];
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
    <span class="logo">⬡ Cortex</span>
    <div class="header-actions">
      <button class="icon-btn" onclick={clearChat} title="Clear chat">↺</button>
      <button class="icon-btn" onclick={() => (showSettings = !showSettings)} title="Settings">
        ⚙
      </button>
    </div>
  </header>

  {#if showSettings}
    <Settings onClose={() => (showSettings = false)} />
  {:else}
    <main>
      <div class="messages" bind:this={messagesEl}>
        {#if messages.length === 0}
          <div class="empty">Start a conversation</div>
        {/if}
        {#each messages as msg}
          <div class="message {msg.role}">
            <div class="bubble">{msg.content}</div>
          </div>
        {/each}
        {#if loading}
          <div class="message assistant">
            <div class="bubble thinking">…</div>
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
          placeholder="Message… (Enter to send, Shift+Enter for newline)"
          rows="3"
          disabled={loading}
        ></textarea>
        <button onclick={send} disabled={loading || !input.trim()}>Send</button>
      </div>
    </main>
  {/if}
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
  }

  .icon-btn {
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

  .icon-btn:hover {
    color: #e8e8ec;
    border-color: #555;
  }

  main {
    display: flex;
    flex-direction: column;
    flex: 1;
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
  }

  .message.user {
    justify-content: flex-end;
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
