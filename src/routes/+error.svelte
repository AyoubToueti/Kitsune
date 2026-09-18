<script lang="ts">
  import { page } from "$app/state";
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