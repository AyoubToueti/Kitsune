<script lang="ts">
  /**
   * An in-app video player.
   *
   * The source is owned by the caller: this component only renders the
   * element and its loading/error states, so the watch page can swap the URL
   * without the player knowing where it came from.
   *
   * Why `<video>` and not a bundled player: on Linux the Tauri webview is
   * WebKitGTK, whose media stack is GStreamer, so it plays the same
   * containers mpv does. What it will NOT render is an embedded ASS subtitle
   * track, which is why the watch page also offers an external player.
   */

  let {
    src,
    poster,
    title,
  }: {
    /** Stream URL. `undefined` renders the empty state. */
    src?: string;
    /** Shown until the first frame arrives. */
    poster?: string;
    /** Accessible name for the player. */
    title?: string;
  } = $props();

  /** Set by the element's own error event. */
  let failed = $state(false);
  /** True until the element reports it can play. */
  let buffering = $state(true);

  // A new source is a new load, so the previous failure and buffering state
  // must not carry over -- otherwise a second attempt would show the first
  // attempt's error.
  $effect(() => {
    void src;
    failed = false;
    buffering = true;
  });
</script>

{#if src}
  <div
    class="relative aspect-video w-full overflow-hidden rounded-xl border border-border-subtle bg-black"
  >
    <!-- Subtitles here live inside the container (often ASS, which this
         element cannot render) and are served with it, so there is no
         separate track URL to point at. The watch page offers an external
         player for those; a captions element would have nothing to load. -->
    <!-- svelte-ignore a11y_media_has_caption -->
    <video
      {src}
      {poster}
      title={title ?? "Video player"}
      controls
      preload="metadata"
      class="h-full w-full"
      oncanplay={() => (buffering = false)}
      onerror={() => {
        buffering = false;
        failed = true;
      }}
    ></video>

    {#if buffering && !failed}
      <p
        class="pointer-events-none absolute inset-0 flex items-center justify-center text-sm text-white/80"
      >
        Buffering…
      </p>
    {/if}

    {#if failed}
      <div
        class="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-black/85 p-4 text-center"
      >
        <p class="text-sm text-white">Could not play this file here.</p>
        <p class="text-xs text-white/70">
          The container or codec may be unsupported. Try the external player.
        </p>
      </div>
    {/if}
  </div>
{:else}
  <div
    class="flex aspect-video w-full items-center justify-center rounded-xl border border-dashed border-border-subtle bg-surface-hover"
  >
    <p class="text-sm text-ink-muted">Load a torrent to start watching.</p>
  </div>
{/if}