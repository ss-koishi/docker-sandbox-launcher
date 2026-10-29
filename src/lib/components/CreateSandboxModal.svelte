<script lang="ts">
  import { onMount } from 'svelte';
  import { createSandbox, ensureWorkspaceDir, errorMessage, listAgents, listKits } from '$lib/api';
  import { settings } from '$lib/settings.svelte';
  import { isMixin, kitLabel, type Kit, type LogLine, type WorkspaceMount } from '$lib/types';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';
  import PathInput from './PathInput.svelte';

  type Props = {
    onclose: () => void;
    oncreated: () => void;
  };

  let { onclose, oncreated }: Props = $props();

  const CUSTOM = '__custom__';
  /** `sbx create --name` の制約に合わせた簡易チェック。 */
  const NAME_RE = /^[A-Za-z0-9][A-Za-z0-9.-]+$/;

  let agents = $state<string[]>([]);
  let kits = $state<Kit[]>([]);
  let loadError = $state<string | null>(null);

  let agentChoice = $state('');
  let customRef = $state('');
  let name = $state('');
  let primary = $state('');
  let extras = $state<WorkspaceMount[]>([]);
  let selected = $state<string[]>([]);

  let running = $state(false);
  let logs = $state<LogLine[]>([]);
  let runError = $state<string | null>(null);
  let logBox = $state<HTMLPreElement | null>(null);

  const agent = $derived(agentChoice === CUSTOM ? customRef.trim() : agentChoice);
  const nameValid = $derived(NAME_RE.test(name.trim()));
  const canSubmit = $derived(!running && agent !== '' && nameValid);
  const mixins = $derived(kits.filter(isMixin));

  /** 選択中の kit のうち、別のエージェントを要求しているもの。 */
  const conflicts = $derived(
    mixins.filter(
      (k) => selected.includes(k.path) && k.requires_agent && k.requires_agent !== agent
    )
  );

  onMount(() => void load());

  async function load() {
    try {
      const [a, k] = await Promise.all([listAgents(), listKits().catch(() => [])]);
      agents = a;
      kits = k;
      if (!agentChoice && a.length > 0) agentChoice = a.includes('claude') ? 'claude' : a[0];
    } catch (e) {
      loadError = errorMessage(e);
    }
  }

  function toggleKit(path: string) {
    selected = selected.includes(path) ? selected.filter((p) => p !== path) : [...selected, path];
  }

  async function makeWorkspaceDir() {
    runError = null;
    try {
      primary = await ensureWorkspaceDir(name.trim());
    } catch (e) {
      runError = errorMessage(e);
    }
  }

  async function submit() {
    running = true;
    runError = null;
    logs = [];

    const workspaces: WorkspaceMount[] = [];
    if (primary.trim()) workspaces.push({ path: primary.trim(), readonly: false });
    workspaces.push(...extras.filter((m) => m.path.trim()));

    try {
      await createSandbox({ agent, name: name.trim(), workspaces, kits: selected }, (line) => {
        logs = [...logs, line];
        queueMicrotask(() => logBox?.scrollTo({ top: logBox.scrollHeight }));
      });
      oncreated();
    } catch (e) {
      runError = errorMessage(e);
    } finally {
      running = false;
    }
  }
</script>

<Modal title="Sandbox を作成" dismissible={!running} {onclose}>
  {#if loadError}
    <p class="error">{loadError}</p>
  {/if}

  <div class="field">
    <span class="label">エージェント</span>
    <select bind:value={agentChoice} disabled={running}>
      {#each agents as a (a)}
        <option value={a}>{a}</option>
      {/each}
      <option value={CUSTOM}>（kit 参照を直接指定）</option>
    </select>
    {#if agentChoice === CUSTOM}
      <input
        type="text"
        bind:value={customRef}
        disabled={running}
        spellcheck="false"
        placeholder="./my-kit, docker.io/org/kit:1.0.0, git+https://…"
      />
    {/if}
    <span class="hint">sandbox の土台になる workload です。</span>
  </div>

  <div class="field">
    <span class="label">sandbox 名</span>
    <input
      type="text"
      bind:value={name}
      disabled={running}
      spellcheck="false"
      placeholder="my-project"
    />
    {#if name.trim() && !nameValid}
      <span class="hint danger">
        2 文字以上で、英数字から始まり、英数字・ハイフン・ピリオドのみが使えます。
      </span>
    {/if}
  </div>

  <div class="field">
    <span class="label">workspace（primary）</span>
    <PathInput bind:value={primary} disabled={running} placeholder="~/sbx-workspaces/my-project" />
    <div class="row">
      <button type="button" onclick={makeWorkspaceDir} disabled={running || !nameValid}>
        {settings.current.workspace_root}/{name.trim() || '<名前>'} を作成して使う
      </button>
    </div>
    <span class="hint">
      sandbox 内にもホストと同じ絶対パスでマウントされます。空にすると bind mount なしで作成します。
    </span>
  </div>

  <div class="field">
    <span class="label">追加マウント</span>
    {#each extras as _mount, i (i)}
      <div class="row mount">
        <PathInput bind:value={extras[i].path} disabled={running} />
        <label class="ro">
          <input type="checkbox" bind:checked={extras[i].readonly} disabled={running} />
          読み取り専用
        </label>
        <button
          type="button"
          class="icon danger"
          onclick={() => (extras = extras.filter((_, j) => j !== i))}
          disabled={running}
          aria-label="この行を削除"
        >
          <Icon name="close" size={15} />
        </button>
      </div>
    {/each}
    <div class="row">
      <button
        type="button"
        onclick={() => (extras = [...extras, { path: '', readonly: true }])}
        disabled={running}
      >
        <span class="btn-inner"><Icon name="plus" size={15} /> ディレクトリを追加</span>
      </button>
    </div>
  </div>

  <div class="field">
    <span class="label">kits（mixin）</span>
    {#if mixins.length === 0}
      <span class="hint">
        {settings.current.kits_dir} に kit が見つかりません。設定からディレクトリを確認してください。
      </span>
    {:else}
      <div class="kits">
        {#each mixins as kit (kit.path)}
          <label class="kit">
            <input
              type="checkbox"
              checked={selected.includes(kit.path)}
              onchange={() => toggleKit(kit.path)}
              disabled={running}
            />
            <span class="kit-name">{kitLabel(kit)}</span>
            <code class="muted">{kit.id}</code>
            {#if kit.requires_agent}
              <span class="chip" class:warning={kit.requires_agent !== agent}>
                requires: {kit.requires_agent}
              </span>
            {/if}
          </label>
        {/each}
      </div>
    {/if}
    {#if conflicts.length > 0}
      <span class="hint warning">
        選択中の kit が別のエージェントを要求しています（{conflicts
          .map((k) => `${kitLabel(k)} → ${k.requires_agent}`)
          .join('、')}）。作成時に sbx 側で弾かれる可能性があります。
      </span>
    {/if}
  </div>

  {#if logs.length > 0 || running}
    <div class="field">
      <span class="label">実行ログ</span>
      <pre class="log" bind:this={logBox}>{#each logs as line, i (i)}<span
            class:err={line.stream === 'stderr'}>{line.text}
</span>{/each}</pre>
    </div>
  {/if}

  {#if runError}
    <p class="error">{runError}</p>
  {/if}

  {#snippet footer()}
    <button type="button" onclick={onclose} disabled={running}>キャンセル</button>
    <button type="button" class="primary" onclick={submit} disabled={!canSubmit}>
      {running ? '作成中…' : '作成'}
    </button>
  {/snippet}
</Modal>

<style>
  .mount {
    margin-bottom: 6px;
  }

  .mount :global(.row) {
    flex: 1;
  }

  .ro {
    display: flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
    font-size: 13px;
  }

  .btn-inner {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .hint.danger {
    color: var(--red);
  }

  .hint.warning {
    color: var(--amber);
  }

  .kits {
    max-height: 190px;
    overflow: auto;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .kit {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 7px;
    border-radius: var(--radius);
  }

  .kit:hover {
    background: var(--row-hover);
  }

  .kit-name {
    font-weight: 500;
  }

  .log {
    margin: 0;
    padding: 10px 12px;
    max-height: 220px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-all;
    background: var(--surface-alt);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .log .err {
    color: var(--amber);
  }
</style>
