<script lang="ts">
  import { onMount } from 'svelte';
  import { errorMessage, listSandboxes, openSandboxTerminal, removeSandboxes } from '$lib/api';
  import { TERMINAL_LABELS, type Sandbox, type TerminalInfo } from '$lib/types';
  import CreateSandboxModal from './CreateSandboxModal.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let sandboxes = $state<Sandbox[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let query = $state('');
  let showCreate = $state(false);
  let removing = $state<Sandbox | null>(null);
  let removeBusy = $state(false);
  /** ターミナルを開こうとしている sandbox 名。 */
  let opening = $state<string | null>(null);
  let openedNotice = $state<string | null>(null);
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;

  const visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return sandboxes;
    return sandboxes.filter(
      (sb) =>
        sb.name.toLowerCase().includes(q) ||
        sb.agent.toLowerCase().includes(q) ||
        sb.workspaces.some((w) => w.toLowerCase().includes(q))
    );
  });

  onMount(() => {
    void refresh();
    return () => clearTimeout(noticeTimer);
  });

  /** `sbx run` を走らせるディレクトリ。primary workspace があればそこ。 */
  function primaryWorkspace(sb: Sandbox): string | undefined {
    const first = sb.workspaces[0];
    return first ? mount(first).path : undefined;
  }

  async function openTerminal(sb: Sandbox) {
    opening = sb.name;
    openedNotice = null;
    clearTimeout(noticeTimer);
    try {
      const used = (await openSandboxTerminal(sb.name, primaryWorkspace(sb))) as TerminalInfo['effective'];
      error = null;
      openedNotice = `${sb.name} を ${TERMINAL_LABELS[used] ?? used} で開きました`;
      noticeTimer = setTimeout(() => (openedNotice = null), 6000);
    } catch (e) {
      error = errorMessage(e);
    } finally {
      opening = null;
    }
  }

  async function refresh() {
    loading = true;
    try {
      sandboxes = await listSandboxes();
      error = null;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      loading = false;
    }
  }

  async function confirmRemove() {
    if (!removing) return;
    removeBusy = true;
    try {
      await removeSandboxes([removing.name]);
      removing = null;
      await refresh();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      removeBusy = false;
    }
  }

  /** `/path:ro` を表示用に分解する。 */
  function mount(raw: string) {
    return raw.endsWith(':ro')
      ? { path: raw.slice(0, -3), readonly: true }
      : { path: raw, readonly: false };
  }

  function formatDate(value: string | null) {
    if (!value) return '—';
    const d = new Date(value);
    return Number.isNaN(d.getTime())
      ? value
      : d.toLocaleString('ja-JP', { dateStyle: 'short', timeStyle: 'short' });
  }
</script>

<div class="head">
  <div>
    <h1 class="page-title">Sandboxes</h1>
    <p class="page-subtitle">
      ローカルの sandbox を作成・削除します（{sandboxes.length} 件）
    </p>
  </div>
</div>

<div class="toolbar">
  <div class="search">
    <Icon name="search" size={15} />
    <input type="text" bind:value={query} placeholder="名前・エージェント・workspace で検索" />
  </div>
  <div class="spacer"></div>
  <button type="button" class="icon" onclick={refresh} disabled={loading} title="再読み込み" aria-label="再読み込み">
    <Icon name="refresh" size={16} />
  </button>
  <button type="button" class="primary" onclick={() => (showCreate = true)}>
    <span class="btn-inner"><Icon name="plus" size={15} /> 作成</span>
  </button>
</div>

{#if error}
  <p class="error">{error}</p>
{/if}

{#if openedNotice}
  <p class="notice"><Icon name="check" size={16} /><span>{openedNotice}</span></p>
{/if}

<div class="surface table-wrap">
  {#if loading && sandboxes.length === 0}
    <p class="empty">読み込み中…</p>
  {:else if visible.length === 0}
    <p class="empty">
      {#if sandboxes.length === 0}
        sandbox がありません。「作成」から追加してください。
      {:else}
        「{query}」に一致する sandbox はありません。
      {/if}
    </p>
  {:else}
    <table class="table">
      <thead>
        <tr>
          <th>名前</th>
          <th>状態</th>
          <th>エージェント</th>
          <th>workspace</th>
          <th>作成</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each visible as sb (sb.id)}
          <tr>
            <td class="name">{sb.name}</td>
            <td>
              <span class="status" class:running={sb.status === 'running'}>{sb.status}</span>
            </td>
            <td class="muted">{sb.agent}</td>
            <td>
              <ul class="mounts">
                {#each sb.workspaces as raw (raw)}
                  {@const m = mount(raw)}
                  <li>
                    <code>{m.path}</code>
                    {#if m.readonly}<span class="chip">ro</span>{/if}
                  </li>
                {:else}
                  <li class="muted">bind mount なし</li>
                {/each}
              </ul>
              {#if sb.workspace_missing}
                <span class="chip warning">workspace 不明</span>
              {/if}
            </td>
            <td class="muted nowrap">{formatDate(sb.created_at)}</td>
            <td class="actions">
              <button
                type="button"
                class="icon"
                onclick={() => openTerminal(sb)}
                disabled={opening !== null}
                title="ターミナルで開く (sbx run)"
                aria-label="{sb.name} をターミナルで開く"
              >
                <Icon name="terminal" size={16} />
              </button>
              <button
                type="button"
                class="icon danger"
                onclick={() => (removing = sb)}
                title="削除"
                aria-label="{sb.name} を削除"
              >
                <Icon name="trash" size={16} />
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if showCreate}
  <CreateSandboxModal
    onclose={() => (showCreate = false)}
    oncreated={() => {
      showCreate = false;
      void refresh();
    }}
  />
{/if}

{#if removing}
  <Modal title="Sandbox を削除" dismissible={!removeBusy} onclose={() => (removing = null)}>
    <p>
      <strong>{removing.name}</strong> を削除します。コンテナと sandbox の状態は消え、この操作は取り消せません。
    </p>
    <p class="muted">ホストの workspace ディレクトリと、保持対象の永続ボリュームは残ります。</p>
    {#snippet footer()}
      <button type="button" onclick={() => (removing = null)} disabled={removeBusy}>
        キャンセル
      </button>
      <button type="button" class="destructive" onclick={confirmRemove} disabled={removeBusy}>
        {removeBusy ? '削除中…' : '削除'}
      </button>
    {/snippet}
  </Modal>
{/if}

<style>
  /* 見出しはページの導入。ツールバー以下の操作群とは距離を置く。 */
  .head {
    margin-bottom: var(--space-group);
  }

  /* ツールバーは直下のテーブルを操作するものなので、表にくっつける。 */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: var(--space-snug);
  }

  .notice {
    margin: 0 0 var(--space-snug);
  }

  .search {
    position: relative;
    display: flex;
    align-items: center;
    width: min(340px, 50%);
    color: var(--text-dim);
  }

  .search :global(svg) {
    position: absolute;
    left: 9px;
    pointer-events: none;
  }

  .search input {
    padding-left: 30px;
  }

  .btn-inner {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .table-wrap {
    overflow: hidden;
  }

  .mounts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .mounts li {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .mounts code {
    word-break: break-all;
  }

  .nowrap {
    white-space: nowrap;
  }
</style>
