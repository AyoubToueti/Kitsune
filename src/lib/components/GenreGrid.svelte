<script lang="ts">
  let { genres }: { genres: string[] } = $props();

  /**
   * Genre names can contain spaces ("Slice of Life"), so the path segment is
   * encoded. The route decodes it on the way back in.
   */
  function href(genre: string): string {
    return `/genre/${encodeURIComponent(genre)}`;
  }
</script>

{#if genres.length}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Browse by genre</h2>

    <ul class="flex flex-wrap gap-2">
      {#each genres as genre (genre)}
        <li>
          <a
            href={href(genre)}
            class="inline-block rounded-full border border-border-subtle bg-surface-hover px-3 py-1 text-sm text-ink-muted transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            {genre}
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/if}