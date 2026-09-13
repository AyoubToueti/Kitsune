<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";

  let { anime }: { anime: Anime[] } = $props();

  /** Zero-padded rank, so the badge column stays aligned past nine. */
  function rank(position: number): string {
    return String(position).padStart(2, "0");
  }

  function title(item: Anime): string {
    return displayTitle(item.title) ?? "Untitled";
  }
</script>

{#if anime.length}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight text-accent">
      Trending
    </h2>

    <ul class="flex gap-4 overflow-x-auto pb-2">
      {#each anime as item, i (item.id)}
        <li class="relative flex shrink-0 items-end gap-2">
          <!-- The rank, set large and rotated so it reads bottom-to-top up
               the left edge. `writing-mode` plus a 180° turn is what makes
               the text run upwards rather than downwards. -->
          <div class="flex h-48 items-center">
            <span
              class="text-4xl font-bold leading-none text-ink-faint"
              style="writing-mode: vertical-rl; transform: rotate(180deg);"
            >
              {rank(i + 1)}
            </span>
          </div>

          <a
            href={`/anime/${item.id}`}
            aria-label={title(item)}
            class="group block w-32 shrink-0 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <div
              class="relative aspect-[2/3] overflow-hidden rounded-lg bg-surface-hover"
            >
              {#if item.coverImage}
                <img
                  src={item.coverImage}
                  alt=""
                  loading="lazy"
                  class="h-full w-full object-cover transition-transform duration-200 group-hover:scale-105"
                />
              {:else}
                <div
                  class="flex h-full w-full items-center justify-center text-xs text-ink-faint"
                >
                  No cover
                </div>
              {/if}
            </div>

            <!-- Title runs up the poster's left edge, matching the reference,
                 and is clamped so a long name cannot stretch the row. -->
            <p
              class="mt-2 line-clamp-2 text-xs font-medium text-ink group-hover:text-accent"
              data-testid="rail-title"
            >
              {title(item)}
            </p>
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/if}