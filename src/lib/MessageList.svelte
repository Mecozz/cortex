<script lang="ts">
  interface Message {
    id: string;
    role: "user" | "assistant";
    content: string;
    provider?: string;
    status?: "queued" | "sending" | "done" | "canceled" | "stopped";
  }

  let { messages, loading, onCancel } = $props<{
    messages: Message[];
    loading: boolean;
    onCancel: (id: string) => void;
  }>();
</script>

{#if messages.length === 0}
  <div class="empty">Start a conversation</div>
{/if}
{#each messages as msg (msg.id)}
  <div class="message {msg.role}" class:dim={msg.status === "canceled"}>
    <div class="bubble">{msg.content}</div>
    {#if msg.role === "user" && msg.status === "queued"}
      <span class="tag"
        >&#x23F3; queued &middot;
        <button class="cancel" onclick={() => onCancel(msg.id)}>cancel</button>
      </span>
    {:else if msg.status === "canceled"}
      <span class="tag">canceled</span>
    {:else if msg.status === "stopped"}
      <span class="tag">&#x23F9; stopped</span>
    {:else if msg.role === "assistant" && msg.provider}
      <span class="provider-tag">{msg.provider}</span>
    {/if}
  </div>
{/each}
{#if loading}
  <div class="message assistant">
    <div class="bubble thinking">...</div>
  </div>
{/if}

<style>
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

  .tag {
    font-size: 11px;
    color: #888;
    margin-top: 3px;
    margin-right: 2px;
  }

  .message.user .tag {
    align-self: flex-end;
  }

  .message.dim .bubble {
    opacity: 0.45;
    text-decoration: line-through;
  }

  button.cancel {
    background: none;
    border: none;
    color: #f87171;
    cursor: pointer;
    font-size: 11px;
    padding: 0;
    text-decoration: underline;
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
</style>
