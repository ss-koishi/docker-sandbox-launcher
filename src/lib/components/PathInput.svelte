<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';

  type Props = {
    value: string;
    placeholder?: string;
    title?: string;
    disabled?: boolean;
  };

  let { value = $bindable(), placeholder = '', title = 'ディレクトリを選択', disabled = false }: Props =
    $props();

  async function pick() {
    const picked = await open({
      directory: true,
      multiple: false,
      title,
      defaultPath: value || undefined
    });
    if (typeof picked === 'string') value = picked;
  }
</script>

<div class="row">
  <input type="text" bind:value {placeholder} {disabled} spellcheck="false" />
  <button type="button" onclick={pick} {disabled}>選択…</button>
</div>
