<script lang="ts">
  import { onMount } from 'svelte';
  import { detectSbx, detectTerminal, errorMessage } from '$lib/api';
  import { settings } from '$lib/settings.svelte';
  import {
    TERMINAL_LABELS,
    type AppSettings,
    type SbxInfo,
    type TerminalInfo,
    type TerminalTarget
  } from '$lib/types';
  import Icon from './Icon.svelte';
  import PathInput from './PathInput.svelte';

  const TERMINAL_OPTIONS: { value: TerminalTarget; label: string }[] = [
    { value: 'auto', label: '自動（herdr が動いていれば herdr、なければ Ghostty）' },
    { value: 'herdr', label: 'herdr（workspace を作成）' },
    { value: 'ghostty', label: 'Ghostty（新しいウィンドウ）' },
    { value: 'custom', label: 'カスタムコマンド' }
  ];

  const CUSTOM_PLACEHOLDER = 'open -na Ghostty.app --args -e {command}';

  const ENV_PLACEHOLDER = `ANTHROPIC_VERTEX_PROJECT_ID=my-gcp-project
CLOUD_ML_REGION=global`;

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
  let term = $state<TerminalInfo | null>(null);

  let noticeTimer: ReturnType<typeof setTimeout> | undefined;

  const dirty = $derived(
    (Object.keys(draft) as (keyof AppSettings)[]).some(
      (key) => draft[key] !== settings.current[key]
    )
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
    term = await detectTerminal().catch(() => null);
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

  <div class="field">
    <span class="label">sbx に渡す環境変数</span>
    <textarea
      bind:value={draft.extra_env}
      spellcheck="false"
      rows="4"
      placeholder={ENV_PLACEHOLDER}
    ></textarea>
    <span class="hint">
      1 行に 1 つ <code>KEY=VALUE</code>。<code>#</code> 始まりの行は無視します。
      作成・削除・ターミナル起動を含む、すべての sbx 呼び出しに渡します。
      <br />
      エージェントによってはホスト側の環境変数を要求します
      （<code>claude-vertex</code> の <code>ANTHROPIC_VERTEX_PROJECT_ID</code> など）。これは
      kit の spec.yml や sbxenv.yaml の <code>env:</code> では代用できません。どちらも
      サンドボックス側のスコープで、チェックはホストの sbx が行うためです。
    </span>
  </div>

  <hr />

  <div class="field">
    <span class="label">ターミナル連携</span>
    <select bind:value={draft.terminal}>
      {#each TERMINAL_OPTIONS as opt (opt.value)}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
    <span class="hint">
      sandbox 一覧の端末ボタンから <code>sbx run --name &lt;名前&gt;</code> を実行する先です。
      {#if term}
        現在の宛先: <strong>{TERMINAL_LABELS[term.effective] ?? term.effective}</strong>
        （herdr: {term.herdr_running ? '稼働中' : '停止'} ／ Ghostty:
        {term.ghostty_installed ? '導入済み' : '未検出'}）
      {/if}
    </span>
  </div>

  {#if draft.terminal === 'auto' || draft.terminal === 'herdr'}
    <div class="field">
      <span class="label">herdr のパス</span>
      <input type="text" bind:value={draft.herdr_path} spellcheck="false" placeholder="（自動検出）" />
      <span class="hint">
        sandbox 名のワークスペースを作り、その pane で sbx run を実行します。
        {#if term?.herdr_path}
          現在: <code>{term.herdr_path}</code>
        {/if}
      </span>
    </div>

    <label class="field checkbox">
      <input type="checkbox" bind:checked={draft.focus_terminal} />
      <span>
        herdr で開いたあと Ghostty を前面に出す
        <span class="hint">
          herdr の TUI は別のターミナル上で動くため、見える場所まで持ち上げます。Ghostty 以外を使っている場合は外してください。
        </span>
      </span>
    </label>
  {/if}

  {#if draft.terminal === 'custom'}
    <div class="field">
      <span class="label">カスタムコマンド</span>
      <input
        type="text"
        bind:value={draft.terminal_command}
        spellcheck="false"
        placeholder={CUSTOM_PLACEHOLDER}
      />
      <span class="hint">
        <code>{'{command}'}</code> が実行するコマンドに置き換わり、<code>sh -c</code> に渡されます。
      </span>
    </div>
  {/if}

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

  hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 4px 0 18px;
  }

  .checkbox {
    flex-direction: row;
    align-items: flex-start;
    gap: 8px;
  }

  .checkbox input {
    margin-top: 3px;
  }

  .checkbox .hint {
    display: block;
    color: var(--text-dim);
    font-size: 12px;
    line-height: 1.5;
  }

  .actions {
    margin-top: 4px;
  }
</style>
