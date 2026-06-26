<script lang="ts">
  let { onClose } = $props<{ onClose: () => void }>();
</script>

<div class="how">
  <div class="how-header">
    <h2>&#x2728; How Cortex Works</h2>
    <button class="close-btn" onclick={onClose}>&#x2715;</button>
  </div>

  <div class="how-body">
    <p class="intro">
      Every message flows through a memory layer before it reaches the model, and
      every reply quietly teaches Cortex something new.
    </p>

    <h3>The live path — what happens when you send a message</h3>
    <div class="flow">
      <div class="node you">
        <span class="ic">&#x1F464;</span>
        <div><b>You</b><small>type a message</small></div>
      </div>
      <div class="arrow">&#x2193;</div>

      <div class="node inject">
        <span class="ic">&#x1F3AF;</span>
        <div>
          <b>Inject</b>
          <small>searches your memory for facts relevant to this message and
            folds them in (plus your persona)</small>
        </div>
      </div>
      <div class="arrow">&#x2193;</div>

      <div class="node provider">
        <span class="ic">&#x1F500;</span>
        <div>
          <b>Provider</b>
          <small>Claude (your subscription) &middot; API key &middot; Ollama
            &mdash; with fallback if one is down</small>
        </div>
      </div>
      <div class="arrow">&#x2193;</div>

      <div class="node you">
        <span class="ic">&#x1F4AC;</span>
        <div><b>Reply</b><small>shown to you, in context</small></div>
      </div>
    </div>

    <h3>The background loop — how Cortex learns</h3>
    <div class="flow">
      <div class="node capture">
        <span class="ic">&#x26A1;</span>
        <div>
          <b>Capture</b>
          <small>after each turn, pulls out facts, preferences, and tasks you
            mentioned</small>
        </div>
      </div>
      <div class="arrow">&#x2193;</div>

      <div class="node brain">
        <span class="ic">&#x1F9E0;</span>
        <div>
          <b>Brain (SQLite)</b>
          <small>facts, messages, relationships, and tasks &mdash; a single local
            file, yours forever</small>
        </div>
      </div>
      <div class="arrow">&#x2193;</div>

      <div class="node librarian">
        <span class="ic">&#x1F319;</span>
        <div>
          <b>Librarian</b>
          <small>runs hourly: builds the relationship graph, fades stale facts,
            summarizes old conversations</small>
        </div>
      </div>
      <div class="loopback">&#x21BA; feeds richer memory back into Inject</div>
    </div>

    <h3>Always there</h3>
    <div class="chips">
      <span class="chip">&#x1F511; Vault &mdash; encrypted keys</span>
      <span class="chip">&#x1F4BE; Backup &mdash; snapshots + restore</span>
      <span class="chip">&#x1F7E2; Health &mdash; live module status</span>
      <span class="chip">&#x1F527; Tools &mdash; forge &amp; run</span>
      <span class="chip">&#x1F512; Privacy &mdash; local-only / no-save modes</span>
    </div>

    <p class="foot">
      Nothing leaves your machine except the message you send to your chosen
      model. The brain lives only on this computer.
    </p>
  </div>
</div>

<style>
  .how {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .how-header {
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

  .how-body {
    flex: 1;
    overflow-y: auto;
    padding: 18px 20px 28px;
  }

  .intro {
    color: #aaa;
    font-size: 13px;
    line-height: 1.6;
    margin-bottom: 22px;
  }

  h3 {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #777;
    font-weight: 600;
    margin: 24px 0 12px;
  }

  .flow {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .node {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    max-width: 460px;
    background: #16161c;
    border: 1px solid #2a2a32;
    border-left-width: 3px;
    border-radius: 10px;
    padding: 12px 14px;
  }

  .node .ic {
    font-size: 22px;
    line-height: 1;
    flex-shrink: 0;
  }

  .node b {
    color: #e8e8ec;
    font-size: 14px;
    display: block;
  }

  .node small {
    color: #999;
    font-size: 12px;
    line-height: 1.45;
    display: block;
    margin-top: 2px;
  }

  .arrow {
    color: #4a4a55;
    font-size: 18px;
    line-height: 1;
    margin: 6px 0;
  }

  .you {
    border-left-color: #a78bfa;
  }
  .inject {
    border-left-color: #38bdf8;
  }
  .provider {
    border-left-color: #34d399;
  }
  .capture {
    border-left-color: #fb923c;
  }
  .brain {
    border-left-color: #f472b6;
  }
  .librarian {
    border-left-color: #fbbf24;
  }

  .loopback {
    margin-top: 10px;
    color: #fbbf24;
    font-size: 12px;
    opacity: 0.85;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .chip {
    background: #16161c;
    border: 1px solid #2a2a32;
    border-radius: 999px;
    padding: 6px 12px;
    font-size: 12px;
    color: #bbb;
  }

  .foot {
    margin-top: 24px;
    color: #777;
    font-size: 12px;
    line-height: 1.6;
    border-top: 1px solid #1e1e24;
    padding-top: 16px;
  }
</style>
