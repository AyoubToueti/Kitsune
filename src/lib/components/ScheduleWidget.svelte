<script lang="ts">
  import { displayTitle, type ScheduledEpisode } from "$lib/types";
  import { formatTime, groupByDay } from "$lib/schedule";

  let { entries = [] }: { entries?: ScheduledEpisode[] } = $props();

  const days = $derived(groupByDay(entries));

  // Which day tab is showing. Defaults to the first group, which is the
  // soonest day, so the widget opens on something relevant.
  let selected = $state(0);

  // Clamped rather than reset: when `entries` changes the day count can
  // shrink, and a stale index would render an empty panel.
  const active = $derived(
    days.length === 0 ? null : days[Math.min(selected, days.length - 1)],
  );

  function title(entry: ScheduledEpisode): string {
    return displayTitle(entry.anime.title) ?? "Untitled";
  }
</script>

{#if days.length && active}
  <section class="mt-8">
    <h2 class="mb-3 text-lg font-semibold tracking-tight text-accent">
      Estimated schedule
    </h2>

    <!-- Day tabs. Horizontally scrollable so a long window does not wrap into
         a block of buttons. -->
    <div class="mb-3 flex gap-2 overflow-x-auto pb-1">
      {#each days as day, i (day.key)}
        <button
          type="button"
          onclick={() => (selected = i)}
          aria-pressed={i === selected}
          class="flex min-w-[4.5rem] shrink-0 flex-col items-center rounded-lg px-3 py-1.5 text-xs transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {i ===
          selected
            ? 'bg-accent text-white'
            : 'bg-surface-hover text-ink-muted hover:text-ink'}"
        >
          <span class="font-medium">{day.weekday}</span>
          <span class="text-[10px] opacity-80">{day.shortDate}</span>
        </button>
      {/each}
    </div>

    <ul class="divide-y divide-border-subtle">
      {#each active.entries as entry (`${entry.anime.id}-${entry.airingAt}`)}
        <li>
          <a
            href={`/anime/${entry.anime.id}`}
            class="flex items-center gap-3 py-2 transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <time
              datetime={new Date(entry.airingAt * 1000).toISOString()}
              class="w-12 shrink-0 text-xs tabular-nums text-ink-faint"
            >
              {formatTime(entry.airingAt)}
            </time>

            <span class="min-w-0 flex-1 truncate text-sm text-ink">
              {title(entry)}
            </span>

            {#if entry.episode != null}
              <span class="shrink-0 rounded bg-surface-hover px-2 py-0.5 text-xs text-ink-muted">
                Episode {entry.episode}
              </span>
            {/if}
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/if}