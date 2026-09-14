<script lang="ts">
  import { formatSeason } from "$lib/season";
  import { displayTitle, type Anime } from "$lib/types";

  let {
    anime,
    startRank = 1,
  }: {
    anime: Anime[];
    /** Rank of the first item, so a later page continues the numbering. */
    startRank?: number;
  } = $props();

  /**
   * A stable hue per genre.
   *
   * The reference colours each genre differently. Deriving the hue from the
   * name keeps a genre the same colour everywhere without a palette that would
   * need maintaining as the genre list changes.
   */
  function genreStyle(genre: string): string {
    let hash = 0;
    for (const char of genre) {
      hash = (hash * 31 + char.charCodeAt(0)) % 360;
    }
    return `background-color: hsl(${hash}, 45%, 24%); color: hsl(${hash}, 80%, 76%)`;
  }

  /** Only the first few fit on one line; more would wrap and change row height. */
  function shownGenres(item: Anime): string[] {
    return item.genres.slice(0, 5);
  }
</script>

<ol class="divide-y divide-border-subtle">
  {#each anime as item, i (item.id)}
    <li>
      <a
        href={`/anime/${item.id}`}
        class="flex items-center gap-4 rounded-lg px-3 py-3 transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        <!-- Decorative: the list is already ordered, so announcing "one" adds
             nothing a screen reader user cannot infer from position. -->
        <span
          aria-hidden="true"
          data-testid="rank"
          class="w-6 shrink-0 text-right text-sm font-semibold text-ink-faint"
        >
          {startRank + i}
        </span>

        <div class="aspect-[2/3] w-12 shrink-0 overflow-hidden rounded bg-surface-hover">
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
          <h3 class="truncate text-sm font-medium text-ink">
            {displayTitle(item.title) ?? "Untitled"}
          </h3>

          {#if item.genres.length}
            <div class="mt-1 flex flex-wrap gap-1">
              {#each shownGenres(item) as genre (genre)}
                <span
                  class="rounded-full px-2 py-0.5 text-[10px]"
                  style={genreStyle(genre)}
                >
                  {genre}
                </span>
              {/each}
            </div>
          {/if}
        </div>

        {#if item.averageScore != null}
          <div class="hidden w-24 shrink-0 text-right sm:block">
            <p class="text-sm font-semibold text-score">{item.averageScore}%</p>
            {#if item.popularity != null}
              <p class="text-[10px] text-ink-faint">
                {item.popularity.toLocaleString()} users
              </p>
            {/if}
          </div>
        {/if}

        {#if item.format || item.episodeCount != null}
          <div class="hidden w-28 shrink-0 lg:block">
            {#if item.format}
              <p class="text-sm text-ink">{item.format}</p>
            {/if}
            {#if item.episodeCount != null}
              <p class="text-[10px] text-ink-faint">
                {item.episodeCount} episodes
              </p>
            {/if}
          </div>
        {/if}

        {#if formatSeason(item.season, item.seasonYear)}
          <div class="hidden w-28 shrink-0 lg:block">
            <p class="text-sm text-ink">
              {formatSeason(item.season, item.seasonYear)}
            </p>
            {#if item.status}
              <p class="text-[10px] text-ink-faint">{item.status}</p>
            {/if}
          </div>
        {/if}
      </a>
    </li>
  {/each}
</ol>