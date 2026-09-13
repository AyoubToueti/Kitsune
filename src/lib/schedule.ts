// Pure helpers for presenting a schedule.
//
// Kept out of the component so the day-boundary, range and formatting rules can
// be tested without rendering anything.

/**
 * Days before today shown in the strip, so a viewer can look back at what
 * already aired.
 */
export const SCHEDULE_PAST_DAYS = 7;

/** Days after today shown in the strip. */
export const SCHEDULE_FUTURE_DAYS = 21;

/**
 * One day in the strip.
 *
 * Carries its own time bounds, because the strip is built without any data: the
 * tabs are pure date arithmetic and only the selected day is fetched.
 */
export interface ScheduleDay {
  /** Local calendar date as `YYYY-MM-DD`, usable as a keyed-each key. */
  key: string;
  /** "Today", "Tomorrow", or a short weekday and date. */
  label: string;
  /** Short weekday for the tabs, e.g. "Sun". */
  weekday: string;
  /** Short date for the tabs, e.g. "Sep 13". */
  shortDate: string;
  /** Start of this local day, in unix seconds. Inclusive. */
  from: number;
  /** End of this local day, in unix seconds. Inclusive. */
  to: number;
}

/** The day strip, plus where today sits in it. */
export interface ScheduleWindow {
  days: ScheduleDay[];
  /** Index of today within `days`, for the default selection. */
  todayIndex: number;
}

/** Midnight at the start of `date`'s local day. */
function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

/**
 * `date` shifted by `days`, at the same local time of day.
 *
 * Going through the constructor rather than adding to the day number is what
 * normalises month and year rollover.
 */
function addDays(date: Date, days: number): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
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
 * Build the day strip.
 *
 * Every day in the range is present regardless of whether anything airs. That
 * keeps the strip a stable width: deriving the days from data would make the
 * tabs appear and disappear as responses arrive, and a quiet day would vanish
 * rather than showing as empty.
 */
export function buildDayStrip(
  now: Date = new Date(),
  pastDays: number = SCHEDULE_PAST_DAYS,
  futureDays: number = SCHEDULE_FUTURE_DAYS,
): ScheduleWindow {
  const today = startOfDay(now);
  const tomorrow = addDays(today, 1);
  const first = addDays(today, -pastDays);
  const count = pastDays + 1 + futureDays;

  const days: ScheduleDay[] = [];

  for (let i = 0; i < count; i++) {
    const date = addDays(first, i);
    const next = addDays(date, 1);

    days.push({
      key: dayKey(date),
      label: labelFor(date, today, tomorrow),
      weekday: date.toLocaleDateString(undefined, { weekday: "short" }),
      shortDate: date.toLocaleDateString(undefined, {
        day: "numeric",
        month: "short",
      }),
      from: Math.floor(date.getTime() / 1000),
      // Exclusive midnight minus a second, so the whole final day is covered
      // without bleeding into the next one.
      to: Math.floor(next.getTime() / 1000) - 1,
    });
  }

  // Day 0 sits `pastDays` before today, so today's index is exactly `pastDays`.
  return { days, todayIndex: pastDays };
}

/** A broadcast time in the viewer's own timezone, e.g. "21:30". */
export function formatTime(airingAt: number): string {
  return new Date(airingAt * 1000).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });
}