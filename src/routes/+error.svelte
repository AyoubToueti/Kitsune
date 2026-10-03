<script lang="ts">
  import { onMount } from "svelte";

  import { page } from "$app/state";
  import { logFrontendError } from "$lib/api/diagnostics";

  // A route-level error is caught here rather than by `window.onerror`, so it
  // would otherwise be shown to the reader but never recorded. Forward it to
  // the backend log once on mount.
  onMount(() => {
    const message = page.error?.message;
    if (message) {
      void logFrontendError("error", `route error: ${message}`);
    }
  });
</script>

<!--
  A route-level error boundary.

  Without this, an exception thrown while a page renders aborts Svelte's DOM
  update and the previous content stays on screen -- which is how a bad field on
  a detail response once left the loading skeleton up forever with the error
  visible only in the console. Rendering the error here makes that failure
  legible instead of silent.
-->
<div class="py-16 text-center">
  <p class="text-lg font-semibold text-ink">Something went wrong.</p>
  <p class="mt-2 text-sm text-ink-muted">{page.error?.message}</p>
  <a
    href="/"
    class="mt-6 inline-block rounded-full bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
  >
    Back to home
  </a>
</div>