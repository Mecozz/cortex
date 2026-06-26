<script lang="ts">
  let { value = $bindable(), processing, onSend, onStop } = $props<{
    value: string;
    processing: boolean;
    onSend: () => void;
    onStop: () => void;
  }>();

  function key(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      onSend();
    }
  }
</script>

<div class="input-row">
  <textarea
    bind:value
    onkeydown={key}
    placeholder="Message... (Enter to send, Shift+Enter for newline)"
    rows="3"
  ></textarea>
  {#if processing}
    <button class="stop-btn" onclick={onStop}>&#x23F9; Stop</button>
  {/if}
  <button class="send" onclick={onSend} disabled={!value.trim()}>Send</button>
</div>

<style>
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

  button.send {
    background: #5b21b6;
    border: none;
    border-radius: 8px;
    color: #fff;
    cursor: pointer;
    font-size: 14px;
    padding: 0 20px;
    transition: background 0.15s;
  }

  button.send:hover:not(:disabled) {
    background: #6d28d9;
  }

  button.send:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  button.stop-btn {
    background: #7f1d1d;
    border: none;
    border-radius: 8px;
    color: #fff;
    cursor: pointer;
    font-size: 14px;
    padding: 0 16px;
  }

  button.stop-btn:hover {
    background: #991b1b;
  }
</style>
