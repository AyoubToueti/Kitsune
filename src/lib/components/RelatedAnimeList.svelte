<script lang="ts">
  import { displayTitle, type RelatedAnime } from "$lib/types";

  let { relations = [] }: { relations?: RelatedAnime[] } = $props();

  /**
   * Turn the provider's SCREAMING_SNAKE relation type into a readable badge.
   *
   * `SIDE_STORY` -> "Side Story", `SEQUEL` -> "Sequel". Kept generic rather than
   * a lookup table so a type this app has never seen still renders sensibly.
   */
  function relationLabel(type: string): string {
    return type
      .toLowerCase()
      .split("_")
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(" ");
  }
</script>

{#if relations.length > 0}
  <div>
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Related Anime</h2>

    <!-- The list scrolls inside a box capped at three rows (3 × 88px rows +
         2 × 8px gaps = 280px), so a franchise with many relations stays a
         sidebar rather than pushing the rest of the page down. -->
    <div
      data-testid="related-scroller"
      class="max-h-[17.5rem] overflow-y-auto pr-1"
    >
      <ul class="flex flex-col gap-2">
        {#each relations as item (item.id)}
          <li>
            <a
              href={`/anime/${item.id}`}
              class="group flex gap-3 rounded-lg p-2 transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              <div
                class="aspect-[2/3] w-12 shrink-0 overflow-hidden rounded bg-surface-hover"
              >
                {#if item.coverImage}
                  <img
                    src={item.coverImage}
                    alt=""
                    loading="lazy"
                    class="h-full w-full object-cover"
                  />
                {/if}
              </div>

              <div class="min-w-0 flex-1">
                <h3
                  class="line-clamp-2 text-sm font-medium text-ink transition-colors group-hover:text-accent"
                >
                  {displayTitle(item.title) ?? "Untitled"}
                </h3>

                <div
                  class="mt-1.5 flex flex-wrap items-center gap-x-1.5 gap-y-1 text-xs"
                >
                  <span
                    class="rounded bg-accent/20 px-1.5 py-0.5 font-medium text-accent"
                  >
                    {relationLabel(item.relationType)}
                  </span>

                  {#if item.episodeCount != null}
                    <span
                      class="rounded bg-surface-hover px-1.5 py-0.5 text-ink-muted"
                    >
                      {item.episodeCount}
                    </span>
                  {/if}

                  {#if item.format}
                    <span class="text-ink-faint" aria-hidden="true">•</span>
                    <span class="text-ink-faint">{item.format}</span>
                  {/if}
                </div>
              </div>
            </a>
          </li>
        {/each}
      </ul>
    </div>
  </div>
{/if}