<script lang="ts">
  import { onMount } from 'svelte';
  import { detectSbx, errorMessage } from '$lib/api';
  import { settings } from '$lib/settings.svelte';
  import type { SbxInfo } from '$lib/types';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import KitsView from '$lib/components/KitsView.svelte';
  import Logo from '$lib/components/Logo.svelte';
  import SandboxesView from '$lib/components/SandboxesView.svelte';
  import SettingsView from '$lib/components/SettingsView.svelte';

  const TABS: { id: TabId; label: string; icon: IconName }[] = [
    { id: 'sandboxes', label: 'Sandboxes', icon: 'box' },
    { id: 'kits', label: 'Kits', icon: 'layers' }
  ];

  type TabId = 'sandboxes' | 'kits' | 'settings';

  let tab = $state<TabId>('sandboxes');
  let bootError = $state<string | null>(null);
  let sbx = $state<SbxInfo | null>(null);
  /** 設定保存後に各画面を作り直すためのキー。 */
  let reloadKey = $state(0);

  onMount(async () => {
    try {
      await settings.load();
    } catch (e) {
      bootError = errorMessage(e);
    }
    void probeSbx();
  });

  async function probeSbx() {
    try {
      sbx = await detectSbx();
    } catch {
      sbx = null;
    }
  }
</script>

<div class="shell">
  <!-- macOS の信号機ボタンはヘッダーに重ねる。空き領域はウィンドウのドラッグに使う。 -->
  <header class="topbar" data-tauri-drag-region>
    <div class="brand">
      <Logo />
      <span class="wordmark"><strong>docker</strong> sandbox launcher</span>
    </div>
    <div class="spacer" data-tauri-drag-region></div>
    {#if sbx}
      <span class="chip" title={sbx.path}>{sbx.version || 'sbx'}</span>
    {:else}
      <span class="chip warning">sbx 未検出</span>
    {/if}
    <button
      type="button"
      class="icon"
      class:active={tab === 'settings'}
      onclick={() => (tab = 'settings')}
      title="設定"
      aria-label="設定"
    >
      <Icon name="settings" size={18} />
    </button>
  </header>

  <div class="body">
    <nav class="sidebar">
      {#each TABS as t (t.id)}
        <button type="button" class="nav" class:active={tab === t.id} onclick={() => (tab = t.id)}>
          <Icon name={t.icon} size={18} />
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
        <!--
          設定画面は key の外に置く。中に入れると保存した瞬間に作り直されて、
          保存完了の表示が消えてしまう。
        -->
        {#if tab === 'settings'}
          <SettingsView
            onsaved={() => {
              reloadKey += 1;
              void probeSbx();
            }}
          />
        {:else}
          {#key reloadKey}
            {#if tab === 'sandboxes'}
              <SandboxesView />
            {:else}
              <KitsView />
            {/if}
          {/key}
        {/if}
      {/if}
    </main>
  </div>
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--content-bg);
  }

  .topbar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: var(--header-height);
    /* 左は信号機ボタンの分を空ける。 */
    padding: 0 12px 0 80px;
    background: var(--header-bg);
    border-bottom: 1px solid var(--border);
    flex: none;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .wordmark {
    font-size: 15px;
    letter-spacing: -0.01em;
    color: var(--text);
  }

  .wordmark strong {
    font-weight: 700;
  }

  .topbar .icon.active {
    background: var(--blue-tint);
    color: var(--blue);
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .sidebar {
    width: var(--sidebar-width);
    flex: none;
    padding: 10px 8px;
    background: var(--sidebar-bg);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 10px;
    border: none;
    background: none;
    border-radius: var(--radius);
    color: var(--text-dim);
    font-weight: 400;
    text-align: left;
  }

  .nav:hover:not(.active) {
    background: var(--row-hover);
    color: var(--text);
  }

  .nav.active {
    background: var(--blue-tint);
    color: var(--blue);
    font-weight: 500;
  }

  main {
    flex: 1;
    min-width: 0;
    overflow: auto;
    padding: 20px 24px 28px;
    background: var(--content-bg);
  }
</style>
