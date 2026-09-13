<script lang="ts">
  let { count }: { count?: number } = $props();

  // Synthesise 1..=count so the user can see the episode structure even
  // though no source is configured. A missing/zero count gets a message
  // instead of an empty grid, which would look broken.
  const hasGrid = $derived(count != null && count > 0);
  const episodes = $derived(
    hasGrid ? Array.from({ length: count! }, (_, i) => i + 1) : [],
  );
</script>

{#if hasGrid}
  <div>
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Episodes</h2>
    <ul class="grid grid-cols-4 gap-2 sm:grid-cols-6 md:grid-cols-8">
      {#each episodes as ep (ep)}
        <li>
          <button
            type="button"
            disabled
            title="No source configured"
            aria-label="Episode {ep} — no source configured"
            class="flex h-12 items-center justify-center rounded-lg border border-border-subtle bg-surface-hover text-sm font-medium text-ink-muted opacity-60"
          >
            {ep}
          </button>
        </li>
      {/each}
    </ul>
  </div>
{:else}
  <div>
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Episodes</h2>
    <p class="text-sm text-ink-faint">Episode count unknown.</p>
  </div>
{/if}