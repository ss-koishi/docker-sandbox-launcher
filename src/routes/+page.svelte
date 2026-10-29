<script lang="ts">
  import { onMount } from 'svelte';
  import { errorMessage } from '$lib/api';
  import { settings } from '$lib/settings.svelte';
  import KitsView from '$lib/components/KitsView.svelte';
  import SandboxesView from '$lib/components/SandboxesView.svelte';
  import SettingsView from '$lib/components/SettingsView.svelte';

  const TABS = [
    { id: 'sandboxes', label: 'サンドボックス' },
    { id: 'kits', label: 'キット' },
    { id: 'settings', label: '設定' }
  ] as const;

  type TabId = (typeof TABS)[number]['id'];

  let tab = $state<TabId>('sandboxes');
  let bootError = $state<string | null>(null);
  /** 設定保存後に各画面を作り直すためのキー。 */
  let reloadKey = $state(0);

  onMount(async () => {
    try {
      await settings.load();
    } catch (e) {
      bootError = errorMessage(e);
    }
  });
</script>

<div class="app">
  <nav>
    {#each TABS as t (t.id)}
      <button type="button" class:active={tab === t.id} onclick={() => (tab = t.id)}>
        {t.label}
      </button>
    {/each}
  </nav>

  <main>
    {#if bootError}
      <p class="error">{bootError}</p>
    {:else if !settings.loaded}
      <p class="muted">読み込み中…</p>
    {:else}
      {#key reloadKey}
        {#if tab === 'sandboxes'}
          <SandboxesView />
        {:else if tab === 'kits'}
          <KitsView />
        {:else}
          <SettingsView onsaved={() => (reloadKey += 1)} />
        {/if}
      {/key}
    {/if}
  </main>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  nav {
    display: flex;
    gap: 4px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }

  nav button {
    border: none;
    background: none;
    padding: 5px 12px;
    border-radius: 6px;
    color: var(--text-dim);
  }

  nav button.active {
    background: var(--panel-alt);
    color: var(--text);
    font-weight: 600;
  }

  main {
    flex: 1;
    overflow: auto;
    padding: 16px;
  }
</style>
