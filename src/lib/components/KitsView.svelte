<script lang="ts">
  import { onMount } from 'svelte';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  import { errorMessage, listKits, readKitSpec } from '$lib/api';
  import { settings } from '$lib/settings.svelte';
  import { kitLabel, type Kit } from '$lib/types';

  let kits = $state<Kit[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  /** spec を展開中の kit のパス → 原文。 */
  let specs = $state<Record<string, string>>({});

  onMount(() => void refresh());

  async function refresh() {
    loading = true;
    try {
      kits = await listKits();
      error = null;
    } catch (e) {
      kits = [];
      error = errorMessage(e);
    } finally {
      loading = false;
    }
  }

  async function toggleSpec(kit: Kit) {
    if (!kit.spec_file) return;
    if (kit.path in specs) {
      const { [kit.path]: _, ...rest } = specs;
      specs = rest;
      return;
    }
    try {
      specs = { ...specs, [kit.path]: await readKitSpec(kit.spec_file) };
    } catch (e) {
      error = errorMessage(e);
    }
  }
</script>

<header class="toolbar">
  <div class="row">
    <strong>キット</strong>
    <code class="muted">{settings.current.kits_dir}</code>
  </div>
  <div class="row">
    <button type="button" onclick={refresh} disabled={loading}>再読み込み</button>
    <button
      type="button"
      onclick={() => revealItemInDir(settings.current.kits_dir)}
      disabled={!settings.current.kits_dir}>Finder で開く</button
    >
  </div>
</header>

{#if error}
  <p class="error">{error}</p>
{/if}

{#if loading && kits.length === 0}
  <p class="muted">読み込み中…</p>
{:else if kits.length === 0 && !error}
  <p class="muted">
    spec.yml を持つサブディレクトリが見つかりません。設定画面で kits ディレクトリを変更できます。
  </p>
{:else}
  <div class="list">
    {#each kits as kit (kit.path)}
      <article class="panel card">
        <div class="head">
          <div class="row">
            <strong>{kitLabel(kit)}</strong>
            {#if kit.kind}<span class="badge">{kit.kind}</span>{/if}
            {#if kit.requires_agent}
              <span class="badge">requires: {kit.requires_agent}</span>
            {/if}
          </div>
          <div class="row">
            {#if kit.spec_file}
              <button type="button" class="link" onclick={() => toggleSpec(kit)}>
                {kit.path in specs ? 'spec を閉じる' : 'spec を見る'}
              </button>
            {/if}
            <button type="button" onclick={() => revealItemInDir(kit.path)}>Finder</button>
          </div>
        </div>

        <div class="meta muted">
          <code>{kit.path}</code>
        </div>

        {#if kit.error}
          <p class="error">{kit.error}</p>
        {/if}

        {#if kit.path in specs}
          <pre class="spec">{specs[kit.path]}</pre>
        {/if}
      </article>
    {/each}
  </div>
{/if}

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .toolbar code {
    word-break: break-all;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .card {
    padding: 12px 14px;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .meta {
    font-size: 12px;
    margin-top: 4px;
    word-break: break-all;
  }

  .spec {
    margin: 10px 0 0;
    padding: 10px;
    max-height: 320px;
    overflow: auto;
    background: var(--panel-alt);
    border: 1px solid var(--border);
    border-radius: 6px;
  }
</style>
