<script lang="ts">
  import { displayTitle, type Anime } from "$lib/types";
  import { stripHtml } from "$lib/text";

  let { anime }: { anime: Anime[] } = $props();

  /** How long each slide is shown before advancing. */
  const ADVANCE_MS = 7000;

  /**
   * The synopsis is clamped to this many lines.
   *
   * Together with the fixed container height this is what stops a long
   * description from making one slide taller than the others.
   */
  const SYNOPSIS_LINES = 3;

  let index = $state(0);
  // Auto-advance stops while the user is reading or tabbing through, so a
  // slide cannot change out from under them mid-interaction.
  let paused = $state(false);

  const count = $derived(anime.length);

  /**
   * Reduced motion means no automatic movement and no slide transition.
   *
   * Guarded because `matchMedia` is absent in the test environment, and a
   * missing API should not crash the page — it just means "no preference".
   */
  const prefersReducedMotion = $derived(
    typeof window !== "undefined" &&
      typeof window.matchMedia === "function" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

  function title(item: Anime): string {
    return displayTitle(item.title) ?? "Untitled";
  }

  function synopsis(item: Anime): string | undefined {
    return stripHtml(item.description);
  }

  /** The backdrop: banner if the provider has one, else the portrait cover. */
  function backdrop(item: Anime): string | undefined {
    return item.bannerImage ?? item.coverImage;
  }

  /** Short facts shown inline under the title. */
  function facts(item: Anime): string[] {
    const out: string[] = [];
    if (item.format) out.push(item.format);
    if (item.durationMinutes != null) out.push(`${item.durationMinutes}m`);
    if (item.seasonYear != null) out.push(String(item.seasonYear));
    if (item.episodeCount != null) out.push(`${item.episodeCount} eps`);
    return out;
  }

  /** Step by `delta`, wrapping at both ends. */
  function step(delta: number) {
    if (count === 0) return;
    index = (index + delta + count) % count;
  }

  /** Jump straight to a slide, for the dot controls. */
  function goTo(target: number) {
    if (target < 0 || target >= count) return;
    index = target;
  }

  $effect(() => {
    // Reading `index` here makes it a dependency, so a manual jump restarts
    // the countdown rather than leaving a half-elapsed timer running.
    const _ = index;

    if (count < 2 || paused || prefersReducedMotion) return;

    const timer = setInterval(() => step(1), ADVANCE_MS);
    return () => clearInterval(timer);
  });
</script>

{#if count > 0}
  <!-- The height is fixed rather than content-driven, so slides cannot change
       size as the carousel moves. `overflow-hidden` clips anything that would
       otherwise spill out. -->
  <section
    class="relative h-[22rem] overflow-hidden rounded-xl border border-border-subtle md:h-[26rem]"
    aria-roledescription="carousel"
    aria-label="Featured titles"
    onmouseenter={() => (paused = true)}
    onmouseleave={() => (paused = false)}
    onfocusin={() => (paused = true)}
    onfocusout={() => (paused = false)}
  >
    <!-- Slides sit side by side in a flex row and the row is shifted as a
         whole. That is what produces a slide rather than a fade. -->
    <div
      data-testid="carousel-track"
      class="flex h-full {prefersReducedMotion
        ? ''
        : 'transition-transform duration-500 ease-out'}"
      style="transform: translateX(-{index * 100}%)"
    >
      {#each anime as item, i (item.id)}
        <!-- Inactive slides are hidden from assistive tech and made inert, so
             they are neither announced nor reachable by Tab. Without this a
             screen reader would read every slide at once. -->
        <article
          class="relative h-full w-full shrink-0"
          aria-hidden={i === index ? undefined : "true"}
          inert={i !== index}
        >
          {#if backdrop(item)}
            <img
              src={backdrop(item)}
              alt=""
              class="absolute inset-0 h-full w-full object-cover"
            />
          {/if}

          <!-- Two scrims: one across for text contrast, one up from the bottom
               so the artwork fades into the page rather than ending abruptly. -->
          <div
            class="absolute inset-0 bg-gradient-to-r from-surface via-surface/85 to-surface/20"
          ></div>
          <div
            class="absolute inset-0 bg-gradient-to-t from-surface/90 via-transparent to-transparent"
          ></div>

          <div
            class="relative flex h-full max-w-2xl flex-col justify-center gap-3 px-8"
          >
            <p class="text-sm font-medium text-accent">#{i + 1} Spotlight</p>

            <h1 class="text-3xl font-semibold tracking-tight md:text-4xl">
              {title(item)}
            </h1>

            <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-sm">
              {#if item.averageScore != null}
                <span class="font-semibold text-score">{item.averageScore}</span>
              {/if}
              {#each facts(item) as fact (fact)}
                <span class="text-ink-muted">{fact}</span>
              {/each}
            </div>

            {#if (item.genres ?? []).length}
              <ul class="flex flex-wrap gap-2">
                {#each (item.genres ?? []).slice(0, 3) as genre (genre)}
                  <li
                    class="rounded-full bg-surface-hover px-2.5 py-0.5 text-xs text-ink-muted"
                  >
                    {genre}
                  </li>
                {/each}
              </ul>
            {/if}

            {#if synopsis(item)}
              <!-- Clamped: a long synopsis is cut with an ellipsis instead of
                   pushing the layout taller. -->
              <p
                data-testid="hero-synopsis"
                class="max-w-xl text-sm text-ink-muted"
                style="display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: {SYNOPSIS_LINES}; overflow: hidden;"
              >
                {synopsis(item)}
              </p>
            {/if}

            <div class="mt-2">
              <a
                href={`/anime/${item.id}`}
                class="inline-block rounded-full bg-accent px-5 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
              >
                View details
              </a>
            </div>
          </div>
        </article>
      {/each}
    </div>

    {#if count > 1}
      <!-- Stacked on the right edge, over the artwork. z-10 keeps them above
           the scrims, which are painted after the track's own content. -->
      <div class="absolute top-1/2 right-4 z-10 flex -translate-y-1/2 flex-col gap-2">
        <button
          type="button"
          onclick={() => step(1)}
          aria-label="Next featured title"
          class="flex h-9 w-9 items-center justify-center rounded-full bg-surface/80 text-ink transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          ›
        </button>
        <button
          type="button"
          onclick={() => step(-1)}
          aria-label="Previous featured title"
          class="flex h-9 w-9 items-center justify-center rounded-full bg-surface/80 text-ink transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          ‹
        </button>
      </div>

      <ul class="absolute bottom-4 left-8 z-10 flex gap-2">
        {#each anime as item, i (item.id)}
          <li>
            <button
              type="button"
              onclick={() => goTo(i)}
              aria-label={`Show featured title ${i + 1} of ${count}`}
              aria-current={i === index ? "true" : undefined}
              class="h-2 w-2 rounded-full transition-colors {i === index
                ? 'bg-accent'
                : 'bg-ink-faint hover:bg-ink-muted'}"
            ></button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}