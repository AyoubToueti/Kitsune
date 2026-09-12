// Small text helpers shared by the UI components.

/** Common HTML entities AniList uses in descriptions. */
const ENTITIES: Record<string, string> = {
  "&quot;": '"',
  "&#039;": "'",
  "&#39;": "'",
  "&amp;": "&",
  "&lt;": "<",
  "&gt;": ">",
  "&mdash;": "—",
  "&ndash;": "–",
  "&hellip;": "…",
  "&nbsp;": " ",
};

/**
 * Turn AniList's light HTML into plain text.
 *
 * Descriptions arrive with `<br>` and occasional `<i>` tags. Rendering them
 * with `{@html}` would inject third-party markup into the app, so the tags
 * are stripped here and the result is rendered as ordinary text.
 *
 * Returns `undefined` when nothing readable is left, so callers can skip the
 * element rather than render an empty paragraph.
 */
export function stripHtml(input: string | undefined | null): string | undefined {
  if (input == null) return undefined;

  let text = input
    .replace(/<br\s*\/?>/gi, " ")
    .replace(/<\/p>/gi, " ")
    .replace(/<[^>]*>/g, "");

  for (const [entity, replacement] of Object.entries(ENTITIES)) {
    text = text.split(entity).join(replacement);
  }

  text = text.replace(/\s+/g, " ").trim();
  return text === "" ? undefined : text;
}