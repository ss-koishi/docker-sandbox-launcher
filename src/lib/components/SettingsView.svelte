<script lang="ts">
  import { onMount } from 'svelte';
  import { detectSbx, errorMessage } from '$lib/api';
  import { settings } from '$lib/settings.svelte';
  import type { AppSettings, SbxInfo } from '$lib/types';
  import Icon from './Icon.svelte';
  import PathInput from './PathInput.svelte';

  type Props = {
    /** 設定が変わったら各画面を読み直してもらう。 */
    onsaved: () => void;
  };

  let { onsaved }: Props = $props();

  /** 保存完了の表示を自動で消すまでの時間。 */
  const NOTICE_MS = 6000;

  let draft = $state<AppSettings>({ ...settings.current });
  let saving = $state(false);
  /** 保存が成功した時刻。null なら未保存。 */
  let savedAt = $state<Date | null>(null);
  let error = $state<string | null>(null);

  let sbx = $state<SbxInfo | null>(null);
  let sbxError = $state<string | null>(null);

  let noticeTimer: ReturnType<typeof setTimeout> | undefined;

  const dirty = $derived(
    draft.kits_dir !== settings.current.kits_dir ||
      draft.sbx_path !== settings.current.sbx_path ||
      draft.workspace_root !== settings.current.workspace_root
  );

  onMount(() => {
    void probe();
    return () => clearTimeout(noticeTimer);
  });

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
    savedAt = null;
    clearTimeout(noticeTimer);
    try {
      await settings.save(draft);
      draft = { ...settings.current };
      error = null;
      savedAt = new Date();
      noticeTimer = setTimeout(() => (savedAt = null), NOTICE_MS);
      onsaved();
      await probe();
    } catch (e) {
      error = errorMessage(e);
    } finally {
      saving = false;
    }
  }

  function revert() {
    draft = { ...settings.current };
    savedAt = null;
    clearTimeout(noticeTimer);
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
      直下のサブディレクトリのうち spec.yml を持つものを kit として扱い、sandbox 作成時の選択肢に出します。
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

  {#if savedAt}
    <p class="notice">
      <Icon name="check" size={16} />
      <span>
        設定を保存しました
        <span class="muted">（{savedAt.toLocaleTimeString('ja-JP')}）</span>
      </span>
    </p>
  {/if}

  <div class="row actions">
    <button type="button" class="primary" onclick={save} disabled={saving || !dirty}>
      {saving ? '保存中…' : '保存'}
    </button>
    <button type="button" onclick={revert} disabled={saving || !dirty}>元に戻す</button>
    {#if dirty}
      <span class="muted">未保存の変更があります</span>
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

  .notice {
    margin: 0 0 14px;
  }

  .actions {
    margin-top: 4px;
  }
</style>
