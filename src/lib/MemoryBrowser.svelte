<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface Fact {
    content: string;
    category: string;
    confidence: number;
  }

  interface SearchResult {
    role: string;
    content: string;
  }

  interface Props {
    onClose: () => void;
  }
  let { onClose }: Props = $props();

  let facts: Fact[] = $state([]);
  let query = $state("");
  let results: SearchResult[] = $state([]);
  let loading = $state(false);
  let tab: "facts" | "search" = $state("facts");

  async function loadFacts() {
    loading = true;
    facts = await invoke<Fact[]>("get_facts").catch(() => []);
    loading = false;
  }

  async function search() {
    if (!query.trim()) return;
    loading = true;
    results = await invoke<SearchResult[]>("search_memory", { query }).catch(() => []);
    loading = false;
  }

  loadFacts();
</script>

<div class="panel">
  <div class="panel-header">
    <span>Memory</span>
    <button class="close-btn" onclick={onClose}>×</button>
  </div>

  <div class="tabs">
    <button class="tab" class:active={tab === "facts"} onclick={() => { tab = "facts"; loadFacts(); }}>
      Facts
    </button>
    <button class="tab" class:active={tab === "search"} onclick={() => (tab = "search")}>
      Search
    </button>
  </div>

  {#if tab === "facts"}
    <div class="content">
      {#if loading}
        <div class="empty">Loading...</div>
      {:else if facts.length === 0}
        <div class="empty">No facts stored yet. Have a conversation first.</div>
      {:else}
        {#each facts as fact}
          <div class="fact-item">
            <span class="fact-content">{fact.content}</span>
            <span class="fact-meta">{fact.category} · {Math.round(fact.confidence * 100)}%</span>
          </div>
        {/each}
      {/if}
    </div>
  {:else}
    <div class="search-bar">
      <input
        bind:value={query}
        onkeydown={(e) => e.key === "Enter" && search()}
        placeholder="Search conversations..."
      />
      <button onclick={search} disabled={loading}>Search</button>
    </div>
    <div class="content">
      {#if results.length === 0}
        <div class="empty">Type a query and press Enter.</div>
      {:else}
        {#each results as r}
          <div class="result-item {r.role}">
            <span class="role-badge">{r.role}</span>
            <span class="result-text">{r.content}</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
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

  .tabs {
    display: flex;
    border-bottom: 1px solid #222228;
  }

  .tab {
    flex: 1;
    background: none;
    border: none;
    color: #666;
    cursor: pointer;
    padding: 10px;
    font-size: 13px;
    transition: color 0.15s;
  }

  .tab.active {
    color: #a78bfa;
    border-bottom: 2px solid #a78bfa;
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

  .fact-item {
    background: #1c1c22;
    border: 1px solid #2a2a32;
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .fact-content {
    font-size: 13px;
    color: #e8e8ec;
    line-height: 1.4;
  }

  .fact-meta {
    font-size: 11px;
    color: #555;
  }

  .search-bar {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid #222228;
  }

  .search-bar input {
    flex: 1;
    background: #18181f;
    border: 1px solid #333;
    border-radius: 6px;
    color: #e8e8ec;
    padding: 6px 10px;
    font-size: 13px;
    outline: none;
  }

  .search-bar input:focus {
    border-color: #5b21b6;
  }

  .search-bar button {
    background: #5b21b6;
    border: none;
    border-radius: 6px;
    color: #fff;
    cursor: pointer;
    font-size: 13px;
    padding: 6px 14px;
  }

  .search-bar button:disabled {
    opacity: 0.4;
  }

  .result-item {
    background: #1c1c22;
    border: 1px solid #2a2a32;
    border-radius: 8px;
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .role-badge {
    font-size: 10px;
    text-transform: uppercase;
    color: #555;
    letter-spacing: 0.05em;
  }

  .result-item.user .role-badge {
    color: #7c3aed;
  }

  .result-item.assistant .role-badge {
    color: #2563eb;
  }

  .result-text {
    font-size: 13px;
    color: #c0c0ca;
    line-height: 1.4;
  }
</style>
