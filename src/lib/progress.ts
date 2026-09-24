/**
 * Debounced playback-progress recording for the watch page.
 *
 * Starting an episode should mark it watched on the reader's AniList list, but
 * the play path runs more than once for a single episode: resolving the stream,
 * switching files by hand, and re-selecting the same file all reach it. AniList
 * allows roughly ninety requests a minute, so an unguarded write would both
 * waste that budget and keep re-ordering the reader's list.
 *
 * Kept as its own module rather than inlined in the page so the debounce rule
 * is unit-testable without rendering the whole watch page.
 */

/**
 * Save one work's progress. Rejections are handled by the recorder.
 *
 * `episode` is optional: a release played without a selected episode has no
 * number to record, but the work still belongs on the reader's list. Omitting
 * it marks the work Current without touching a stored progress value.
 */
export type ProgressSaver = (
  animeId: number,
  episode: number | undefined,
) => Promise<unknown>;

export interface ProgressRecorder {
  /**
   * Record that `animeId` started playing, at `episode` when one is known.
   *
   * Returns immediately: the save runs in the background and any failure is
   * swallowed. Callers never await it, because progress is a side effect of
   * watching and must never delay or break playback.
   */
  record(animeId: number, episode?: number): void;
}

/**
 * Build a recorder that writes each (anime, episode) at most once.
 *
 * The pair is the key rather than the episode alone: the same number belongs to
 * a different work on another page, and two works could otherwise share one
 * slot.
 *
 * `onError` is called when a save rejects. The recorder still never throws --
 * progress is a side effect of watching -- but a caller that passes this can
 * surface the failure instead of losing it, which is what the empty catch used
 * to do.
 */
export function createProgressRecorder(
  save: ProgressSaver,
  onError?: (
    animeId: number,
    episode: number | undefined,
    error: unknown,
  ) => void,
): ProgressRecorder {
  const recorded = new Set<string>();

  return {
    record(animeId, episode) {
      // An absent episode still forms a key ("21:undefined"), so a work played
      // repeatedly without a number is written once, like any other.
      const key = `${animeId}:${episode}`;
      // Deduplicated before the save is issued, so overlapping plays of the
      // same episode (a fast double-click, say) cannot both reach the API.
      if (recorded.has(key)) return;
      recorded.add(key);

      void save(animeId, episode).catch((error: unknown) => {
        // Reported BEFORE the key is dropped: a write that fails silently is
        // worse than one that fails loudly, and this is the only place the
        // error surfaces at all. Optional so existing callers are unaffected.
        onError?.(animeId, episode, error);
        // A transient failure drops the key so the next play of this episode
        // tries again, rather than suppressing the write for the whole visit.
        // Still bounded by the reader's own actions, so it cannot spam.
        recorded.delete(key);
      });
    },
  };
}