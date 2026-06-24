<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface Task {
    id: string;
    content: string;
    status: string;
    proj_id: string;
  }

  interface Props {
    onClose: () => void;
  }
  let { onClose }: Props = $props();

  let tasks: Task[] = $state([]);
  let loading = $state(false);

  async function loadTasks() {
    loading = true;
    tasks = await invoke<Task[]>("get_tasks").catch(() => []);
    loading = false;
  }

  async function closeTask(id: string) {
    await invoke("close_task", { taskId: id }).catch(() => {});
    await loadTasks();
  }

  loadTasks();
</script>

<div class="panel">
  <div class="panel-header">
    <span>Tasks</span>
    <button class="close-btn" onclick={onClose}>×</button>
  </div>

  <div class="content">
    {#if loading}
      <div class="empty">Loading...</div>
    {:else if tasks.length === 0}
      <div class="empty">No open tasks. Mention tasks or goals in conversation.</div>
    {:else}
      {#each tasks as task}
        <div class="task-item">
          <span class="task-content">{task.content}</span>
          <button class="done-btn" onclick={() => closeTask(task.id)}>Done</button>
        </div>
      {/each}
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
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .empty {
    color: #444;
    text-align: center;
    margin-top: 40px;
    font-size: 13px;
  }

  .task-item {
    background: #1c1c22;
    border: 1px solid #2a2a32;
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .task-content {
    flex: 1;
    font-size: 13px;
    color: #e8e8ec;
    line-height: 1.4;
  }

  .done-btn {
    background: #1a2e1a;
    border: 1px solid #2d4a2d;
    border-radius: 6px;
    color: #4ade80;
    cursor: pointer;
    font-size: 12px;
    padding: 4px 10px;
    flex-shrink: 0;
    transition: background 0.15s;
  }

  .done-btn:hover {
    background: #1e3a1e;
  }
</style>
