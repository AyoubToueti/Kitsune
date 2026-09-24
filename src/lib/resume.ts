// The one conversion between how progress is STORED and how the watch page
// TAKES it.
//
// AniList's `progress` is a 1-based episode NUMBER: after watching episode 3 it
// reads 3. The watch page's `?ep=` is a 0-based INDEX into its episode list, so
// episode 3 is index 2. Everything that builds a resume link -- the Continue
// Watching row and the floating disc -- goes through here, so the off-by-one
// cannot be fixed in one place and left broken in the other.

/**
 * The watch page's `?ep=` index for a work at `progress` episodes watched.
 *
 * Floored at 0: a work never started (`progress` 0) resumes at the first
 * episode, not at -1.
 */
export function resumeIndex(progress: number): number {
  return Math.max(0, progress - 1);
}