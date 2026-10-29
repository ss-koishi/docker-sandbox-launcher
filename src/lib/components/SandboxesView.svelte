<script lang="ts">
  import { onMount } from 'svelte';
  import { errorMessage, listSandboxes, removeSandboxes } from '$lib/api';
  import type { Sandbox } from '$lib/types';
  import CreateSandboxModal from './CreateSandboxModal.svelte';
  import Modal from './Modal.svelte';

  let sandboxes = $state<Sandbox[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showCreate = $state(false);
  let removing = $state<Sandbox | null>(null);
  let removeBusy = $state(false);

  onMount(() => void refresh());

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
    return Number.isNaN(d.getTime()) ? value : d.toLocaleString('ja-JP');
  }
</script>

<header class="toolbar">
  <div class="row">
    <strong>サンドボックス</strong>
    <span class="muted">{sandboxes.length} 件</span>
  </div>
  <div class="row">
    <button type="button" onclick={refresh} disabled={loading}>再読み込み</button>
    <button type="button" class="primary" onclick={() => (showCreate = true)}>＋ 作成</button>
  </div>
</header>

{#if error}
  <p class="error">{error}</p>
{/if}

{#if loading && sandboxes.length === 0}
  <p class="muted">読み込み中…</p>
{:else if sandboxes.length === 0 && !error}
  <p class="muted">サンドボックスがありません。「＋ 作成」から追加してください。</p>
{:else}
  <div class="list">
    {#each sandboxes as sb (sb.id)}
      <article class="panel card">
        <div class="head">
          <div class="row">
            <strong>{sb.name}</strong>
            <span class="badge" class:running={sb.status === 'running'} class:stopped={sb.status !== 'running'}>
              {sb.status}
            </span>
            <span class="muted">{sb.agent}</span>
            {#if sb.workspace_missing}
              <span class="badge warn">workspace 不明</span>
            {/if}
          </div>
          <button type="button" class="danger" onclick={() => (removing = sb)}>削除</button>
        </div>

        <ul class="mounts">
          {#each sb.workspaces as raw, i (raw)}
            {@const m = mount(raw)}
            <li>
              <code>{m.path}</code>
              {#if i === 0}<span class="badge">primary</span>{/if}
              {#if m.readonly}<span class="badge">ro</span>{/if}
            </li>
          {:else}
            <li class="muted">bind mount なし</li>
          {/each}
        </ul>

        <div class="meta muted">
          作成 {formatDate(sb.created_at)} ／ 最終利用 {formatDate(sb.last_used_at)}
        </div>
      </article>
    {/each}
  </div>
{/if}

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
  <Modal title="サンドボックスを削除" dismissible={!removeBusy} onclose={() => (removing = null)}>
    <p>
      <strong>{removing.name}</strong> を削除します。コンテナと sandbox の状態は消えます（この操作は取り消せません）。
    </p>
    <p class="muted">
      ホストの workspace ディレクトリと、保持対象の永続ボリュームは残ります。
    </p>
    {#snippet footer()}
      <button type="button" onclick={() => (removing = null)} disabled={removeBusy}>キャンセル</button>
      <button type="button" class="primary danger" onclick={confirmRemove} disabled={removeBusy}>
        {removeBusy ? '削除中…' : '削除する'}
      </button>
    {/snippet}
  </Modal>
{/if}

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
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

  .mounts {
    list-style: none;
    margin: 8px 0 6px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .mounts li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .mounts code {
    word-break: break-all;
  }

  .meta {
    font-size: 12px;
  }

  .primary.danger {
    background: var(--danger);
    color: #fff;
  }
</style>
