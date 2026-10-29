<script lang="ts" module>
  /** Docker Desktop と同じく輪郭線のアイコンで揃える。 */
  export const PATHS = {
    box: [
      'M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z',
      'm3.3 7 8.7 5 8.7-5',
      'M12 22V12'
    ],
    layers: [
      'M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z',
      'm22 12.5-9.17 4.16a2 2 0 0 1-1.66 0L2 12.5',
      'm22 17.5-9.17 4.16a2 2 0 0 1-1.66 0L2 17.5'
    ],
    settings: [
      'M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2Z',
      'M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z'
    ],
    trash: [
      'M3 6h18',
      'M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6',
      'M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2',
      'M10 11v6',
      'M14 11v6'
    ],
    refresh: [
      'M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8',
      'M21 3v5h-5',
      'M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16',
      'M8 16H3v5'
    ],
    plus: ['M5 12h14', 'M12 5v14'],
    search: ['M19 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0Z', 'm21 21-4.3-4.3'],
    folder: [
      'M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z'
    ],
    file: [
      'M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z',
      'M14 2v5h6',
      'M16 13H8',
      'M16 17H8'
    ],
    check: ['M20 6 9 17l-5-5'],
    close: ['M18 6 6 18', 'm6 6 12 12'],
    terminal: ['m4 17 6-6-6-6', 'M12 19h8']
  } as const;

  export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
  type Props = {
    name: IconName;
    size?: number;
  };

  let { name, size = 16 }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each PATHS[name] as d (d)}
    <path {d} />
  {/each}
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>
