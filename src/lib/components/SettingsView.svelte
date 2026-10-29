<script lang="ts">
  import { onMount } from 'svelte';
  import { detectSbx, errorMessage } from '$lib/api';
  import { settings } from '$lib/settings.svelte';
  import type { AppSettings, SbxInfo } from '$lib/types';
  import PathInput from './PathInput.svelte';

  type Props = {
    /** 設定が変わったら各画面を読み直してもらう。 */
    onsaved: () => void;
  };

  let { onsaved }: Props = $props();

  let draft = $state<AppSettings>({ ...settings.current });
  let saving = $state(false);
  let saved = $state(false);
  let error = $state<string | null>(null);

  let sbx = $state<SbxInfo | null>(null);
  let sbxError = $state<string | null>(null);

  const dirty = $derived(
    draft.kits_dir !== settings.current.kits_dir ||
      draft.sbx_path !== settings.current.sbx_path ||
      draft.workspace_root !== settings.current.workspace_root
  );

  onMount(() => void probe());

  async function probe() {
    try {
      sbx = await detectSbx();
      sbxError = null;
    } catch (e) {
      sbx = null;
      sbxError = errorMessage(e);
    }
  }

  async function save() {
    saving = true;
    saved = false;
    try {
      await settings.save(draft);
      draft = { ...settings.current };
      error = null;
      saved = true;
      onsaved();
      await probe();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="head">
  <h1 class="page-title">設定</h1>
  <p class="page-subtitle">kit の置き場と sbx の呼び出し先を指定します</p>
</div>

<section class="surface card">
  <div class="field">
    <span class="label">kits ディレクトリ</span>
    <PathInput bind:value={draft.kits_dir} title="kits ディレクトリを選択" />
    <span class="hint">
      直下のサブディレクトリのうち spec.yml を持つものを kit として扱い、作成画面の選択肢に出します。
    </span>
  </div>

  <div class="field">
    <span class="label">workspace の作成先</span>
    <PathInput bind:value={draft.workspace_root} title="workspace の作成先を選択" />
    <span class="hint">作成画面の「〜を作成して使う」でディレクトリを掘る親ディレクトリです。</span>
  </div>

  <div class="field">
    <span class="label">sbx のパス</span>
    <input type="text" bind:value={draft.sbx_path} spellcheck="false" placeholder="（自動検出）" />
    <span class="hint">
      空にすると PATH と Homebrew の標準的な場所から探します。
      {#if sbx}
        現在: <code>{sbx.path}</code>（{sbx.version}）
      {/if}
    </span>
    {#if sbxError}
      <span class="hint danger">{sbxError}</span>
    {/if}
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="row actions">
    <button type="button" class="primary" onclick={save} disabled={saving || !dirty}>
      {saving ? '保存中…' : '保存'}
    </button>
    <button
      type="button"
      onclick={() => {
        draft = { ...settings.current };
        saved = false;
      }}
      disabled={saving || !dirty}
    >
      元に戻す
    </button>
    {#if saved && !dirty}
      <span class="muted">保存しました</span>
    {/if}
  </div>
</section>

<style>
  .head {
    margin-bottom: 16px;
  }

  .card {
    padding: 20px;
    max-width: 640px;
  }

  .hint.danger {
    color: var(--red);
  }

  .actions {
    margin-top: 4px;
  }
</style>
