<script lang="ts">
  import { displayTitle, type ScheduledEpisode } from "$lib/types";
  import { formatTime, groupByDay } from "$lib/schedule";

  let { entries = [] }: { entries?: ScheduledEpisode[] } = $props();

  const days = $derived(groupByDay(entries));

  function title(entry: ScheduledEpisode): string {
    return displayTitle(entry.anime.title) ?? "Untitled";
  }
</script>

{#if days.length}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight">Airing soon</h2>

    <div class="space-y-4">
      {#each days as day (day.key)}
        <div>
          <h3 class="mb-2 text-sm font-medium text-ink-muted">{day.label}</h3>

          <ul class="space-y-2">
            {#each day.entries as entry (`${entry.anime.id}-${entry.airingAt}`)}
              <li class="flex items-baseline gap-3">
                <time
                  datetime={new Date(entry.airingAt * 1000).toISOString()}
                  class="w-12 shrink-0 text-xs tabular-nums text-ink-faint"
                >
                  {formatTime(entry.airingAt)}
                </time>

                <a
                  href={`/anime/${entry.anime.id}`}
                  class="min-w-0 flex-1 truncate text-sm text-ink transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {title(entry)}
                </a>

                {#if entry.episode != null}
                  <span class="shrink-0 text-xs text-ink-faint">EP {entry.episode}</span>
                {/if}
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  </section>
{/if}