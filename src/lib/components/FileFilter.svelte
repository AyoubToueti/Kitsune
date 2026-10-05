<script lang="ts">
  /**
   * A name filter for a torrent's file list.
   *
   * Shown only once a torrent holds more than a handful of files: below that
   * the list is short enough to scan and the box would be noise. The value and
   * the change callback are owned by the caller, so each list keeps its own
   * query rather than the two sharing one.
   */
  let {
    value,
    onInput,
  }: {
    /** The current query. */
    value: string;
    /** Called with the new query as the reader types. */
    onInput: (next: string) => void;
  } = $props();
</script>

<input
  type="search"
  value={value}
  oninput={(event) => onInput(event.currentTarget.value)}
  placeholder="Filter files…"
  aria-label="Filter files"
  data-testid="file-filter"
  class="mb-2 w-full rounded-lg border border-border-subtle bg-surface px-3 py-1.5 text-xs text-ink placeholder:text-ink-faint focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
/>