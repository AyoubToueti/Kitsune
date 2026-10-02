<script lang="ts">
  import { untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { beginLogin, authStatus, rateRecommendation } from "$lib/api/auth";
  import { errorMessage } from "$lib/api/anime";
  import type { RecommendationRating, RecommendedAnime } from "$lib/types";
  import AnimeCard from "./AnimeCard.svelte";

  /**
   * A recommended work plus the credit line the card itself cannot carry.
   *
   * `AnimeCard` shows only the artwork and title; a recommendation also has a
   * vote tally and a suggester, so those ride underneath as a caption rather
   * than being pushed into the card.
   */
  let {
    recommendation,
    fluid = false,
    /**
     * Whether the reader can vote here. The detail page's inline row leaves it
     * off (it is a preview, and voting from a hover rail is not the place); the
     * full recommendations page turns it on.
     */
    votable = false,
    /**
     * The base work these recommendations belong to. Required for a vote:
     * AniList keys a recommendation by the (base, recommended) pair.
     */
    mediaId,
  }: {
    recommendation: RecommendedAnime;
    fluid?: boolean;
    votable?: boolean;
    mediaId?: number;
  } = $props();

  /** Local copy so a vote can update the tally optimistically. */
  // `untrack` because the `$effect` below keeps these in step with the prop;
  // reading it reactively here would warn, and would also fight that sync.
  let rating = $state(untrack(() => recommendation.rating));
  let userRating = $state<RecommendationRating | undefined>(
    untrack(() => recommendation.userRating),
  );
  let voting = $state(false);
  let error = $state<string | null>(null);

  // The card is reused across recommendations as the list grows, so track the
  // prop: without this a recycled card would show the previous work's tally.
  $effect(() => {
    rating = recommendation.rating;
    userRating = recommendation.userRating;
  });

  const { anime, user } = $derived(recommendation);

  /** Whether voting is possible at all: opted in AND given a base work. */
  const canVote = $derived(votable && mediaId !== undefined);

  /** How the tally moves when the reader's vote changes from `from` to `to`. */
  function tallyDelta(
    from: RecommendationRating | undefined,
    to: RecommendationRating,
  ): number {
    const value = (r: RecommendationRating | undefined): number =>
      r === "rateUp" ? 1 : r === "rateDown" ? -1 : 0;
    return value(to) - value(from);
  }

  /**
   * Apply a vote, or clear it when the same arrow is pressed again.
   *
   * Optimistic: the tally moves immediately and reverts if the write fails. A
   * signed-out reader is sent to sign-in instead of hitting a rejected mutation.
   */
  async function vote(direction: "rateUp" | "rateDown"): Promise<void> {
    if (mediaId === undefined || voting) return;

    const signedIn = await authStatus().catch(() => false);
    if (!signedIn) {
      // Launching the browser is a UI action, so the URL is fetched then opened
      // here rather than by the backend.
      await beginLogin()
        .then((url) => openUrl(url))
        .catch(() => {});
      return;
    }

    const next: RecommendationRating =
      userRating === direction ? "noRating" : direction;
    const previousRating = rating;
    const previousUserRating = userRating;

    rating = rating + tallyDelta(userRating, next);
    userRating = next === "noRating" ? undefined : next;
    voting = true;
    error = null;

    try {
      // Trust the server's tally over the guess: other readers vote too.
      rating = await rateRecommendation(mediaId, anime.id, next);
    } catch (err) {
      rating = previousRating;
      userRating = previousUserRating;
      error = errorMessage(err);
    } finally {
      voting = false;
    }
  }
</script>

<div class="flex flex-col gap-2" data-testid="recommendation-card">
  <AnimeCard {anime} {fluid} />

  <!-- Credit line: votes, and who suggested it when the provider knows. The
       vote count is always shown; the name is the only extra context AniList
       offers, so it is worth surfacing when present. -->
  <div class="flex items-center gap-2 px-1 text-[11px] font-medium text-ink-muted">
    {#if canVote}
      <button
        type="button"
        data-testid="vote-up"
        aria-label="Upvote this recommendation"
        aria-pressed={userRating === "rateUp"}
        disabled={voting}
        onclick={() => vote("rateUp")}
        class="flex shrink-0 items-center justify-center rounded p-0.5 transition-colors hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent aria-pressed:text-accent disabled:opacity-50"
      >
        <svg class="h-3.5 w-3.5 fill-current" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 5l7 7h-4v7h-6v-7H5z" />
        </svg>
      </button>
    {/if}

    <span
      class="flex shrink-0 items-center gap-1 text-score"
      data-testid="recommendation-rating"
      title={`${rating} ${rating === 1 ? "upvote" : "upvotes"}`}
    >
      <svg class="h-3 w-3 fill-current" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 4l2.9 5.9 6.5.9-4.7 4.6 1.1 6.5L12 18.8 6.2 21.9l1.1-6.5L2.6 10.8l6.5-.9z" />
      </svg>
      {rating}
    </span>

  {#if error}
    <p class="px-1 text-[10px] text-danger" data-testid="vote-error">{error}</p>
  {/if}

    {#if canVote}
      <button
        type="button"
        data-testid="vote-down"
        aria-label="Downvote this recommendation"
        aria-pressed={userRating === "rateDown"}
        disabled={voting}
        onclick={() => vote("rateDown")}
        class="flex shrink-0 items-center justify-center rounded p-0.5 transition-colors hover:text-danger focus:outline-none focus-visible:ring-2 focus-visible:ring-accent aria-pressed:text-danger disabled:opacity-50"
      >
        <svg class="h-3.5 w-3.5 fill-current" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 19l-7-7h4V5h6v7h4z" />
        </svg>
      </button>
    {/if}

    {#if user}
      <span class="flex min-w-0 items-center gap-1.5" data-testid="recommendation-user">
        {#if user.avatar}
          <img
            src={user.avatar}
            alt=""
            aria-hidden="true"
            loading="lazy"
            class="h-4 w-4 shrink-0 rounded-full object-cover"
          />
        {/if}
        <span class="truncate" title={user.name}>{user.name}</span>
      </span>
    {/if}
  </div>
</div>