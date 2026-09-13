// Pure helpers for presenting a schedule.
//
// Kept out of the component so the day-boundary and ordering rules can be
// tested without rendering anything.

import type { ScheduledEpisode } from "$lib/types";

/** One calendar day's worth of broadcasts. */
export interface ScheduleDay {
  /** Local calendar date as `YYYY-MM-DD`, usable as a keyed-each key. */
  key: string;
  /** "Today", "Tomorrow", or a short weekday and date. */
  label: string;
  /**
   * Short weekday for the day tabs, e.g. "Sun".
   *
   * Separate from `label` because the tabs want a fixed two-part shape
   * (weekday over date) rather than the prose "Today"/"Tomorrow" used in
   * the list.
   */
  weekday: string;
  /** Short date for the day tabs, e.g. "Sep 13". */
  shortDate: string;
  entries: ScheduledEpisode[];
}

/**
 * The local calendar date of a moment, as `YYYY-MM-DD`.
 *
 * Built from the local getters rather than `toISOString()`, which converts to
 * UTC and can shift the date by a day for anyone not on UTC. A broadcast at
 * 23:00 local belongs to that local day, not to tomorrow in UTC.
 */
function dayKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function labelFor(date: Date, today: Date, tomorrow: Date): string {
  const key = dayKey(date);
  if (key === dayKey(today)) return "Today";
  if (key === dayKey(tomorrow)) return "Tomorrow";

  return date.toLocaleDateString(undefined, {
    weekday: "short",
    day: "numeric",
    month: "short",
  });
}

/**
 * Group broadcasts by local calendar day, earliest day first.
 *
 * `now` is injectable so the Today/Tomorrow labels are testable without
 * depending on the clock.
 */
export function groupByDay(
  entries: ScheduledEpisode[],
  now: Date = new Date(),
): ScheduleDay[] {
  const tomorrow = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);

  // Sort defensively. The backend orders by time already, but grouping must
  // not silently depend on that staying true.
  const ordered = [...entries].sort((a, b) => a.airingAt - b.airingAt);

  const groups = new Map<string, ScheduleDay>();

  for (const entry of ordered) {
    const when = new Date(entry.airingAt * 1000);
    const key = dayKey(when);

    let group = groups.get(key);
    if (!group) {
      group = {
        key,
        label: labelFor(when, now, tomorrow),
        weekday: when.toLocaleDateString(undefined, { weekday: "short" }),
        shortDate: when.toLocaleDateString(undefined, {
          day: "numeric",
          month: "short",
        }),
        entries: [],
      };
      groups.set(key, group);
    }
    group.entries.push(entry);
  }

  return [...groups.values()];
}

/** A broadcast time in the viewer's own timezone, e.g. "21:30". */
export function formatTime(airingAt: number): string {
  return new Date(airingAt * 1000).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });
}