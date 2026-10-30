<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

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
  <div class="dialog" role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <h2>{title}</h2>
      <button type="button" class="icon" onclick={onclose} disabled={!dismissible} aria-label="閉じる">
        <Icon name="close" size={16} />
      </button>
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
    background: rgba(0, 8, 77, 0.38);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 28px;
    z-index: 10;
  }

  .dialog {
    width: min(680px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-radius: var(--radius-lg);
    box-shadow: 0 12px 40px rgba(0, 8, 77, 0.28);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 15px 18px;
    border-bottom: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
  }

  .body {
    padding: 22px 24px;
    overflow: auto;
  }

  .body :global(p) {
    margin: 0 0 12px;
  }

  .body :global(p:last-child) {
    margin-bottom: 0;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 18px;
    border-top: 1px solid var(--border);
  }
</style>
