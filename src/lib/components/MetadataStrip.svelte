<script lang="ts">
  import type { Anime } from "$lib/types";

  let { anime }: { anime: Anime } = $props();

  // Build the list of visible chips. Each entry is shown only when its
  // field is present, so sparse data doesn't render empty labels.
  type Chip = { label: string; value: string; class?: string };
  const chips = $derived<Chip[]>([
    ...(anime.format ? [{ label: "Format", value: anime.format }] : []),
    ...(anime.episodeCount != null
      ? [{ label: "Episodes", value: `${anime.episodeCount} eps` }]
      : []),
    ...(anime.durationMinutes != null
      ? [{ label: "Duration", value: `${anime.durationMinutes} min` }]
      : []),
    ...(anime.status ? [{ label: "Status", value: anime.status }] : []),
    ...(anime.seasonYear != null
      ? [{ label: "Year", value: String(anime.seasonYear) }]
      : []),
    ...(anime.averageScore != null
      ? [
          {
            label: "Score",
            value: String(anime.averageScore),
            class: "text-score",
          },
        ]
      : []),
    ...(anime.popularity != null
      ? [
          {
            label: "Popularity",
            value: anime.popularity.toLocaleString(),
          },
        ]
      : []),
  ]);
</script>

{#if chips.length > 0}
  <dl class="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm">
    {#each chips as chip (chip.label)}
      <div class="flex items-center gap-1.5">
        <dt class="text-ink-faint">{chip.label}</dt>
        <dd class={chip.class ?? "text-ink"}>{chip.value}</dd>
      </div>
    {/each}
  </dl>
{/if}