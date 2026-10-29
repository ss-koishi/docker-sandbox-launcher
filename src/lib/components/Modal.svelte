<script lang="ts">
  import type { Snippet } from 'svelte';

  type Props = {
    title: string;
    /** Esc と背景クリックで閉じられるか。処理中は false にする。 */
    dismissible?: boolean;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  };

  let { title, dismissible = true, onclose, children, footer }: Props = $props();

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && dismissible) onclose();
  }
</script>

<svelte:window {onkeydown} />

<div
  class="backdrop"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget && dismissible) onclose();
  }}
>
  <div class="dialog panel" role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <h2>{title}</h2>
      <button type="button" onclick={onclose} disabled={!dismissible} aria-label="閉じる">✕</button>
    </header>
    <div class="body">
      {@render children()}
    </div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 10;
  }

  .dialog {
    width: min(720px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.28);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: 14px;
  }

  header button {
    border: none;
    background: none;
    padding: 2px 6px;
  }

  .body {
    padding: 16px;
    overflow: auto;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
</style>
