<script lang="ts">
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { kitLabel, unmet, type Kit, type McpServer } from '$lib/types';
  import Icon from './Icon.svelte';

  type Props = {
    /** 判定対象の kit。 */
    kits: Kit[];
    /** 登録済み secret の名前。 */
    secrets: string[];
    /** 登録済み MCP サーバー。 */
    mcp: McpServer[];
  };

  let { kits, secrets, mcp }: Props = $props();

  /** 未設定の secret を持つ kit だけを、用意コマンドとともに並べる。 */
  const pending = $derived(
    kits
      .map((kit) => ({ kit, lack: unmet(kit, secrets, mcp) }))
      .filter(({ lack }) => lack.secrets.length + lack.mcp.length + lack.files.length > 0)
      .map(({ kit, lack }) => ({
        kit,
        lack,
        // secret-setup が無い kit には、せめて一般形を出す。
        // sbx が next_step を返すなら、推測より確かなのでそれを足す。
        command: [
          kit.secret_setup?.trim() ||
            lack.secrets.map((n) => `sbx secret set ${n}`).join('\n'),
          ...lack.mcp
            .map((n) => mcp.find((s) => s.name === n)?.next_step)
            .filter((step): step is string => !!step)
        ]
          .filter(Boolean)
          .join('\n')
      }))
  );

  let copied = $state<string | null>(null);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  async function copy(path: string, command: string) {
    await writeText(command);
    copied = path;
    clearTimeout(copyTimer);
    copyTimer = setTimeout(() => (copied = null), 3000);
  }

</script>

{#if pending.length > 0}
  <div class="pending">
    <p class="lead">
      <Icon name="key" size={15} />
      <span>作成前にホスト側の用意が要ります。次を実行してください。</span>
    </p>

    {#each pending as { kit, lack, command } (kit.path)}
      <div class="item">
        <div class="meta">
          <strong>{kitLabel(kit)}</strong>
          {#each lack.secrets as name (name)}
            <span class="chip warning">secret: {name}</span>
          {/each}
          {#each lack.mcp as name (name)}
            <span class="chip warning">
              MCP: {name}
              {mcp.find((s) => s.name === name)?.status ?? '未登録'}
            </span>
          {/each}
          {#each lack.files as path (path)}
            <span class="chip warning">file: {path}</span>
          {/each}
        </div>

        <div class="command">
          <pre>{command}</pre>
          <button
            type="button"
            class="icon"
            onclick={() => copy(kit.path, command)}
            title="コピー"
            aria-label="{kitLabel(kit)} の用意コマンドをコピー"
          >
            <Icon name={copied === kit.path ? 'check' : 'copy'} size={15} />
          </button>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .pending {
    border: 1px solid color-mix(in srgb, var(--amber) 45%, transparent);
    border-left: 3px solid var(--amber);
    background: color-mix(in srgb, var(--amber) 8%, transparent);
    border-radius: var(--radius);
    padding: 13px 15px;
  }

  .lead {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 var(--space-snug);
    font-size: 13px;
  }

  .lead :global(svg) {
    color: var(--amber);
    flex: none;
  }

  .item + .item {
    margin-top: 14px;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 7px;
    margin-bottom: 6px;
    font-size: 13px;
  }

  .command {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 9px 9px 9px 12px;
  }

  .command pre {
    flex: 1;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-all;
    line-height: 1.6;
  }
</style>
