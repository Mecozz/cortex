<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface ModuleHealth {
    module: string;
    status: string;
    message: string | null;
    failures: number;
    disabled: boolean;
  }

  interface BrainStatus {
    overall: string;
    modules: ModuleHealth[];
  }

  interface Props {
    onClose: () => void;
  }
  let { onClose }: Props = $props();

  let status: BrainStatus | null = $state(null);
  let loading = $state(true);

  async function load() {
    loading = true;
    status = await invoke<BrainStatus>("get_brain_status").catch(() => null);
    loading = false;
  }

  async function runLib() {
    await invoke("run_lib_now").catch(() => {});
    await load();
  }

  load();

  const statusColor: Record<string, string> = {
    green: "#4ade80",
    yellow: "#fbbf24",
    red: "#f87171",
  };
</script>

<div class="panel">
  <div class="panel-header">
    <span>Brain Health</span>
    <button class="close-btn" onclick={onClose}>×</button>
  </div>

  <div class="content">
    {#if loading}
      <div class="empty">Loading...</div>
    {:else if !status}
      <div class="empty">Could not load status.</div>
    {:else}
      <div class="overall" style="color: {statusColor[status.overall] ?? '#888'}">
        ● {status.overall.toUpperCase()}
      </div>

      <div class="module-list">
        {#each status.modules as mod}
          <div class="module-row" class:disabled={mod.disabled}>
            <span class="dot" style="color: {statusColor[mod.status] ?? '#888'}">●</span>
            <div class="mod-info">
              <span class="mod-name">{mod.module}</span>
              {#if mod.message}
                <span class="mod-msg">{mod.message}</span>
              {/if}
            </div>
            {#if mod.disabled}
              <span class="cb-badge">CB</span>
            {:else if mod.failures > 0}
              <span class="fail-count">{mod.failures} fail</span>
            {/if}
          </div>
        {/each}
      </div>

      <button class="lib-btn" onclick={runLib}>Run LIB cycle now</button>
    {/if}
  </div>
</div>

<style>
  .panel {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: #0f0f11;
    display: flex;
    flex-direction: column;
    z-index: 10;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid #222228;
    font-weight: 600;
    color: #a78bfa;
  }

  .close-btn {
    background: none;
    border: none;
    color: #888;
    font-size: 20px;
    cursor: pointer;
    line-height: 1;
  }

  .close-btn:hover {
    color: #e8e8ec;
  }

  .content {
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
    font-size: 13px;
  }

  .overall {
    font-size: 18px;
    font-weight: 700;
    text-align: center;
    padding: 12px 0;
  }

  .module-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .module-row {
    display: flex;
    align-items: center;
    gap: 10px;
    background: #1c1c22;
    border: 1px solid #2a2a32;
    border-radius: 8px;
    padding: 8px 12px;
  }

  .module-row.disabled {
    opacity: 0.6;
    border-color: #7f1d1d;
  }

  .dot {
    font-size: 12px;
    flex-shrink: 0;
  }

  .mod-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .mod-name {
    font-size: 13px;
    color: #e8e8ec;
    font-weight: 500;
  }

  .mod-msg {
    font-size: 11px;
    color: #555;
  }

  .cb-badge {
    font-size: 10px;
    background: #7f1d1d;
    color: #fca5a5;
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 600;
  }

  .fail-count {
    font-size: 11px;
    color: #fbbf24;
  }

  .lib-btn {
    background: #1c1c22;
    border: 1px solid #2a2a32;
    border-radius: 8px;
    color: #a78bfa;
    cursor: pointer;
    font-size: 13px;
    padding: 10px;
    transition: background 0.15s;
    margin-top: 8px;
  }

  .lib-btn:hover {
    background: #252530;
  }
</style>
