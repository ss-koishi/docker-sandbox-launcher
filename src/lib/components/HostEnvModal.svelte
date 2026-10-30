<script lang="ts">
  import { onMount } from 'svelte';
  import { errorMessage, readKitHostEnv, writeKitHostEnv } from '$lib/api';
  import { kitLabel, parseEnvKeys, type Kit } from '$lib/types';
  import Modal from './Modal.svelte';

  type Props = {
    kit: Kit;
    onclose: () => void;
    onsaved: () => void;
  };

  let { kit, onclose, onsaved }: Props = $props();

  const PLACEHOLDER = `# この kit を使うときだけホストの sbx に渡す環境変数
ANTHROPIC_VERTEX_PROJECT_ID=my-gcp-project
CLOUD_ML_REGION=global`;

  let content = $state('');
  let original = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let error = $state<string | null>(null);

  const keys = $derived(parseEnvKeys(content));
  const dirty = $derived(content !== original);
  /** 空にして保存すると host-env ファイルごと消える。 */
  const willDelete = $derived(content.trim() === '' && original.trim() !== '');

  onMount(() => void load());

  async function load() {
    try {
      original = await readKitHostEnv(kit.path);
      content = original;
    } catch (e) {
      error = errorMessage(e);
    } finally {
      loading = false;
    }
  }

  async function save() {
    saving = true;
    try {
      await writeKitHostEnv(kit.path, content);
      onsaved();
    } catch (e) {
      error = errorMessage(e);
      saving = false;
    }
  }
</script>

<Modal title="host-env を編集 — {kitLabel(kit)}" dismissible={!saving} {onclose}>
  {#if loading}
    <p class="muted">読み込み中…</p>
  {:else}
    <div class="field editor">
      <textarea bind:value={content} spellcheck="false" rows="12" placeholder={PLACEHOLDER}
      ></textarea>
      <span class="hint">
        1 行に 1 つ <code>KEY=VALUE</code>。<code>#</code> 始まりの行と空行は無視します。
        保存先は <code>{kit.path}/host-env</code> です。
      </span>
    </div>

    <div class="field">
      <span class="label">この kit が渡す環境変数</span>
      {#if keys.length > 0}
        <div class="keys">
          {#each keys as key (key)}
            <span class="chip accent">{key}</span>
          {/each}
        </div>
      {:else}
        <span class="hint">まだありません。</span>
      {/if}
    </div>

    {#if willDelete}
      <p class="notice warn">
        空のまま保存すると <code>host-env</code> ファイルを削除します。
      </p>
    {/if}
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#snippet footer()}
    <button type="button" onclick={onclose} disabled={saving}>キャンセル</button>
    <button type="button" class="primary" onclick={save} disabled={saving || loading || !dirty}>
      {saving ? '保存中…' : '保存'}
    </button>
  {/snippet}
</Modal>

<style>
  /* 入力と、その結果であるキー一覧は一続き。間を詰めて因果を見せる。 */
  .editor {
    margin-bottom: var(--space-snug);
  }

  .keys {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .notice.warn {
    border-left-color: var(--amber);
    border-color: color-mix(in srgb, var(--amber) 45%, transparent);
    background: color-mix(in srgb, var(--amber) 10%, transparent);
    margin: 0;
  }
</style>
