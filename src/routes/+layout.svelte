<script lang="ts">
  import { onMount } from "svelte";

  import "../app.css";
  import NavBar from "$lib/components/NavBar.svelte";
  import NowPlayingDisc from "$lib/components/NowPlayingDisc.svelte";
  import ResumeDisc from "$lib/components/ResumeDisc.svelte";
  import { getSettings } from "$lib/api/settings";
  import { initTheme } from "$lib/theme.svelte";

  let { children } = $props();

  // Apply the stored appearance as early as possible. `initTheme` returns a
  // teardown for the OS-preference listener, kept for the app's life.
  onMount(() => {
    let teardown: (() => void) | null = null;
    getSettings()
      .then((settings) => {
        teardown = initTheme(
          settings.theme === "light" || settings.theme === "dark"
            ? settings.theme
            : "system",
        );
      })
      .catch(() => {
        // A failed read still needs a theme: fall back to following the OS.
        teardown = initTheme("system");
      });

    return () => teardown?.();
  });
</script>

<div class="min-h-screen bg-surface text-ink">
  <NavBar />

  <main class="mx-auto max-w-6xl px-4 py-6">
    {@render children()}
  </main>

  <!-- A sibling of <main>, not a child: it is fixed, and nesting it in the
       max-width container would tie it to that layout. Global, so the reader
       can get back to what they were watching from any page. -->
  <ResumeDisc />
  <!-- The now-playing widget takes the same corner while an episode is
       buffering or playing; ResumeDisc hides itself then (see its `visible`). -->
  <NowPlayingDisc />
</div>