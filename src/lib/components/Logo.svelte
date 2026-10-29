<script lang="ts">
  /** コンテナを積んだマーク。Docker のロゴそのものではなく雰囲気を合わせる用。 */
  let { size = 22 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
  <g fill="currentColor">
    <rect x="3" y="11" width="4.2" height="4.2" rx="0.6" />
    <rect x="8.1" y="11" width="4.2" height="4.2" rx="0.6" />
    <rect x="13.2" y="11" width="4.2" height="4.2" rx="0.6" />
    <rect x="8.1" y="6.1" width="4.2" height="4.2" rx="0.6" />
    <rect x="13.2" y="6.1" width="4.2" height="4.2" rx="0.6" />
  </g>
  <path
    d="M2 17.4c1.4 0 2.1-.9 3.5-.9s2.1.9 3.5.9 2.1-.9 3.5-.9 2.1.9 3.5.9 2.1-.9 3.5-.9c1.2 0 1.8.6 2.5.8"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
  />
</svg>

<style>
  svg {
    flex: none;
    display: block;
    color: var(--blue);
  }
</style>
