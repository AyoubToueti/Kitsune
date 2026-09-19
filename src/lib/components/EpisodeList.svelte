<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { episodeNumber, indexOfEpisode } from "$lib/episode";
  import type { Episode } from "$lib/episodes";

  let {
    episodes = [],
    selected,
    onSelect,
  }: {
    episodes?: Episode[];
    /**
     * Index of the episode to mark as playing. Only meaningful alongside
     * `onSelect`; the list stays a set of links without one.
     */
    selected?: number;
    /**
     * Called with the chosen index instead of opening the entry's stream URL.
     *
     * The watch page passes this so a click picks an episode to play from the
     * reader's own torrent, where the detail page leaves it undefined and the
     * card opens AniList's licensed link as before.
     */
    onSelect?: (index: number) => void;
  } = $props();

  /** How long a jumped-to episode stays highlighted, in milliseconds. */
  const HIGHLIGHT_MS = 3000;

  let query = $state<number | null>(null);
  let notFound = $state<number | null>(null);
  let highlighted = $state<number | null>(null);
  let listEl: HTMLUListElement | null = $state(null);
  let highlightTimer: ReturnType<typeof setTimeout> | undefined;

  /**
   * Best label for an episode.
   *
   * Falls back through the title, the site, then the URL, mirroring
   * `StreamingLinks`: AniList sometimes leaves the title blank, but a card with
   * no caption at all would look broken.
   */
  function label(ep: Episode): string {
    return (
      ep.title ??
      ep.site ??
      ep.url ??
      (ep.number !== undefined ? `Episode ${ep.number}` : "Episode")
    );
  }

  /**
   * Scroll the requested episode into view and highlight it briefly.
   *
   * A number that matches nothing is reported rather than ignored, so a typo
   * reads as "not found" instead of a button that does nothing.
   */
  function jumpToEpisode(event: SubmitEvent): void {
    event.preventDefault();

    if (query == null || !Number.isInteger(query) || query < 1) return;
    const requested = query;

    const index = indexOfEpisode(episodes, requested);
    if (index === -1) {
      notFound = requested;
      return;
    }

    notFound = null;
    highlighted = index;

    const card = listEl?.querySelector<HTMLElement>(
      `[data-episode-index="${index}"]`,
    );
    card?.scrollIntoView({ behavior: "smooth", block: "center" });

    clearTimeout(highlightTimer);
    highlightTimer = setTimeout(() => {
      highlighted = null;
    }, HIGHLIGHT_MS);
  }
</script>

{#if episodes.length > 0}
  <div>
    <div class="mb-3 flex items-center justify-between gap-3">
      <h2 class="text-lg font-semibold tracking-tight">Episodes</h2>

      <!-- Jump-to-episode: a form so pressing Enter works as well as the
           button, and a wrong number says so instead of doing nothing. -->
      <form onsubmit={jumpToEpisode} class="flex shrink-0 items-center gap-2">
        <label for="episode-jump" class="sr-only">Jump to episode number</label>
        <input
          id="episode-jump"
          type="number"
          min="1"
          autocomplete="off"
          placeholder="Episode #"
          bind:value={query}
          oninput={() => (notFound = null)}
          class="w-24 rounded-full border border-border-subtle bg-surface-hover px-3 py-1.5 text-sm text-ink placeholder:text-ink-faint focus:outline-none focus:ring-2 focus:ring-accent [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
        />
        <button
          type="submit"
          class="rounded-full bg-accent px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          Go
        </button>
      </form>
    </div>

    {#if notFound !== null}
      <p class="mb-2 text-xs text-danger" role="status">
        No episode {notFound} in this list.
      </p>
    {/if}

    <!-- The grid scrolls inside a fixed-height viewport rather than
         stretching the page, so a long season stays compact. -->
    <div
      data-testid="episode-scroller"
      class="max-h-[24rem] overflow-y-auto pr-1"
    >
      <ul
        bind:this={listEl}
        data-testid="episode-list"
        class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4"
      >
        {#each episodes as ep, index (ep.number ?? ep.url ?? index)}
          <li data-episode-index={index}>
            <button
              type="button"
              onclick={() => {
                if (onSelect) onSelect(index);
                else if (ep.url) openUrl(ep.url);
              }}
              aria-label={label(ep)}
              aria-current={selected === index ? "true" : undefined}
              data-highlighted={highlighted === index ? "true" : undefined}
              class="group relative block aspect-video w-full overflow-hidden rounded-lg border bg-surface-hover text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {highlighted ===
              index
                ? 'border-accent ring-2 ring-accent'
                : selected === index
                  ? 'border-accent'
                  : 'border-border-subtle hover:border-accent'}"
            >
              {#if ep.thumbnail}
                <img
                  src={ep.thumbnail}
                  alt=""
                  loading="lazy"
                  class="absolute inset-0 h-full w-full object-cover transition-transform duration-200 group-hover:scale-105"
                />
              {/if}

              <!-- A bottom gradient so the caption stays readable over any frame. -->
              <span
                aria-hidden="true"
                class="absolute inset-x-0 bottom-0 h-2/3 bg-gradient-to-t from-black/85 to-transparent"
              ></span>

              <span
                class="absolute inset-x-0 bottom-0 line-clamp-2 px-2 py-1.5 text-xs font-medium text-white"
              >
                {label(ep)}
              </span>
              {#if ep.filler}
                <span
                  class="absolute top-1 right-1 rounded bg-danger/85 px-1.5 py-0.5 text-[10px] font-semibold text-white"
                  title="Anime-original filler"
                >
                  FILLER
                </span>
              {/if}
            </button>
          </li>
        {/each}
      </ul>
    </div>
  </div>
{/if}
