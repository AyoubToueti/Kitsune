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

/** Save one episode's progress. Rejections are handled by the recorder. */
export type ProgressSaver = (
  animeId: number,
  episode: number,
) => Promise<unknown>;

export interface ProgressRecorder {
  /**
   * Record that `episode` of `animeId` started playing.
   *
   * Returns immediately: the save runs in the background and any failure is
   * swallowed. Callers never await it, because progress is a side effect of
   * watching and must never delay or break playback.
   */
  record(animeId: number, episode: number): void;
}

/**
 * Build a recorder that writes each (anime, episode) at most once.
 *
 * The pair is the key rather than the episode alone: the same number belongs to
 * a different work on another page, and two works could otherwise share one
 * slot.
 */
export function createProgressRecorder(save: ProgressSaver): ProgressRecorder {
  const recorded = new Set<string>();

  return {
    record(animeId, episode) {
      const key = `${animeId}:${episode}`;
      // Deduplicated before the save is issued, so overlapping plays of the
      // same episode (a fast double-click, say) cannot both reach the API.
      if (recorded.has(key)) return;
      recorded.add(key);

      void save(animeId, episode).catch(() => {
        // A transient failure drops the key so the next play of this episode
        // tries again, rather than suppressing the write for the whole visit.
        // Still bounded by the reader's own actions, so it cannot spam.
        recorded.delete(key);
      });
    },
  };
}