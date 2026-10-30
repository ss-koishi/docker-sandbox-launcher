<script lang="ts">
  import { onMount } from 'svelte';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  import { errorMessage, listKits, listMcpServers, listSecrets, readKitSpec } from '$lib/api';
  import { settings } from '$lib/settings.svelte';
  import { kitLabel, type Kit } from '$lib/types';
  import HostEnvModal from './HostEnvModal.svelte';
  import SecretSetup from './SecretSetup.svelte';
  import Icon from './Icon.svelte';

  let kits = $state<Kit[]>([]);
  /** 登録済み secret の名前。 */
  let secrets = $state<string[]>([]);
  /** 登録済み MCP サーバーの名前。 */
  let mcpServers = $state<string[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  /** spec を展開中の kit のパス → 原文。 */
  let specs = $state<Record<string, string>>({});
  /** host-env を編集中の kit。 */
  let editing = $state<Kit | null>(null);

  onMount(() => void refresh());

  async function refresh() {
    loading = true;
    try {
      kits = await listKits();
      secrets = await listSecrets().catch(() => []);
      mcpServers = await listMcpServers().catch(() => []);
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

<div class="head">
  <h1 class="page-title">Kits</h1>
  <p class="page-subtitle">
    <code>{settings.current.kits_dir}</code> 配下の kit（{kits.length} 件）
  </p>
</div>

<div class="toolbar">
  <div class="spacer"></div>
  <button
    type="button"
    onclick={() => revealItemInDir(settings.current.kits_dir)}
    disabled={!settings.current.kits_dir}
  >
    <span class="btn-inner"><Icon name="folder" size={15} /> Finder で開く</span>
  </button>
  <button type="button" class="icon" onclick={refresh} disabled={loading} title="再読み込み" aria-label="再読み込み">
    <Icon name="refresh" size={16} />
  </button>
</div>

{#if error}
  <p class="error">{error}</p>
{/if}

<div class="surface table-wrap">
  {#if loading && kits.length === 0}
    <p class="empty">読み込み中…</p>
  {:else if kits.length === 0}
    <p class="empty">
      spec.yml を持つサブディレクトリが見つかりません。設定から kits ディレクトリを変更できます。
    </p>
  {:else}
    <table class="table">
      <thead>
        <tr>
          <th>名前</th>
          <th>種別</th>
          <th>要求エージェント</th>
          <th>host-env</th>
          <th>必要な secret</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each kits as kit (kit.path)}
          <tr>
            <td class="name">
              <div class="name-main">
                {kitLabel(kit)}
                {#if kit.error}
                  <span class="chip warning">spec 読み取り失敗</span>
                {/if}
              </div>
              <code class="path muted">{kit.path}</code>
            </td>
            <td>
              {#if kit.kind}<span class="chip">{kit.kind}</span>{:else}<span class="muted">—</span
                >{/if}
            </td>
            <td class="muted">{kit.requires_agent ?? '—'}</td>
            <td>
              <button
                type="button"
                class="env-cell"
                onclick={() => (editing = kit)}
                title="host-env を編集"
              >
                {#if kit.host_env_keys.length > 0}
                  {#each kit.host_env_keys as key (key)}
                    <span class="chip accent">{key}</span>
                  {/each}
                {:else}
                  <span class="muted">＋ 追加</span>
                {/if}
              </button>
            </td>
            <td>
              {#if kit.required_secrets.length === 0}
                <span class="muted">—</span>
              {:else}
                <span class="secrets">
                  {#each kit.required_secrets as name (name)}
                    <span
                      class="chip"
                      class:warning={!secrets.includes(name)}
                      title={secrets.includes(name) ? '設定済み' : '未設定'}
                    >
                      {secrets.includes(name) ? '✓' : '!'}
                      {name}
                    </span>
                  {/each}
                </span>
              {/if}
            </td>
            <td class="actions">
              <button
                type="button"
                class="icon"
                onclick={() => (editing = kit)}
                title="host-env を編集"
                aria-label="{kitLabel(kit)} の host-env を編集"
              >
                <Icon name="edit" size={16} />
              </button>
              {#if kit.spec_file}
                <button
                  type="button"
                  class="icon"
                  class:on={kit.path in specs}
                  onclick={() => toggleSpec(kit)}
                  title="spec を表示"
                  aria-label="{kitLabel(kit)} の spec を表示"
                >
                  <Icon name="file" size={16} />
                </button>
              {/if}
              <button
                type="button"
                class="icon"
                onclick={() => revealItemInDir(kit.path)}
                title="Finder で開く"
                aria-label="{kitLabel(kit)} を Finder で開く"
              >
                <Icon name="folder" size={16} />
              </button>
            </td>
          </tr>
          {#if kit.error}
            <tr class="detail">
              <td colspan="6"><p class="error">{kit.error}</p></td>
            </tr>
          {/if}
          {#if kit.path in specs}
            <tr class="detail">
              <td colspan="6"><pre class="spec">{specs[kit.path]}</pre></td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if kits.length > 0}
  <div class="secret-setup">
    <SecretSetup {kits} {secrets} mcp={mcpServers} />
  </div>
{/if}

{#if editing}
  <HostEnvModal
    kit={editing}
    onclose={() => (editing = null)}
    onsaved={() => {
      editing = null;
      void refresh();
    }}
  />
{/if}

<style>
  .head {
    margin-bottom: var(--space-group);
  }

  .head code {
    word-break: break-all;
  }

  /* ツールバーは直下のテーブルを操作するものなので、表にくっつける。 */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: var(--space-snug);
  }

  .btn-inner {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .table-wrap {
    overflow: hidden;
  }

  /* パスは識別の補助。名前の下に小さく置いて列を節約する。 */
  .name-main {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .path {
    display: block;
    margin-top: 3px;
    font-size: 11px;
    word-break: break-all;
  }

  .secrets {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .secret-setup {
    margin-top: var(--space-gap);
  }

  /* セルごと押せるようにして、空のときも追加導線にする。 */
  .env-cell {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 3px;
    border: none;
    background: none;
    padding: 2px 4px;
    margin: -2px -4px;
    border-radius: var(--radius);
    font-weight: 400;
    text-align: left;
  }

  .env-cell:hover {
    background: var(--surface-alt);
  }

  .actions :global(button.icon.on) {
    background: var(--blue-tint);
    color: var(--blue);
  }

  .detail td {
    padding-top: 0;
    background: var(--surface-alt);
  }

  .detail:hover {
    background: transparent;
  }

  .spec {
    margin: 0 0 14px;
    padding: 14px 16px;
    max-height: 340px;
    overflow: auto;
    background: var(--content-bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
</style>
