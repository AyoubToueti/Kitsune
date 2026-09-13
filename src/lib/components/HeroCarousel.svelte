<script lang="ts">
  import type { Anime } from "$lib/types";
  import HeroBanner from "./HeroBanner.svelte";

  let { anime }: { anime: Anime[] } = $props();

  /** How long each slide is shown before advancing. */
  const ADVANCE_MS = 7000;

  let index = $state(0);
  // Auto-advance stops while the user is reading or tabbing through, so a
  // slide cannot change out from under them mid-interaction.
  let paused = $state(false);

  const count = $derived(anime.length);
  const current = $derived(anime[index]);

  /**
   * Reduced motion means no automatic movement.
   *
   * Guarded because `matchMedia` is absent in the test environment, and a
   * missing API should not crash the page — it just means "no preference".
   */
  const prefersReducedMotion = $derived(
    typeof window !== "undefined" &&
      typeof window.matchMedia === "function" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

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

{#if current}
  <div
    class="relative"
    role="region"
    aria-roledescription="carousel"
    aria-label="Featured titles"
    onmouseenter={() => (paused = true)}
    onmouseleave={() => (paused = false)}
    onfocusin={() => (paused = true)}
    onfocusout={() => (paused = false)}
  >
    <HeroBanner anime={current} />

    {#if count > 1}
      <!-- Controls sit above the banner. z-10 keeps them clickable, since the
           banner paints a gradient layer over its own backdrop. -->
      <div class="absolute inset-x-0 bottom-0 z-10 flex items-center justify-between px-6 pb-3">
        <button
          type="button"
          onclick={() => step(-1)}
          aria-label="Previous featured title"
          class="rounded-full bg-surface/80 px-3 py-1 text-sm text-ink transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          ‹
        </button>

        <ul class="flex gap-2">
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

        <button
          type="button"
          onclick={() => step(1)}
          aria-label="Next featured title"
          class="rounded-full bg-surface/80 px-3 py-1 text-sm text-ink transition-colors hover:bg-surface-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          ›
        </button>
      </div>
    {/if}
  </div>
{/if}