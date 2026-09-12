<script lang="ts">
  // A plain GET form rather than a JS submit handler.
  //
  // SvelteKit handles GET forms natively, so this works without JavaScript,
  // needs no `$app/navigation` import, and stays testable without mocking
  // SvelteKit's runtime modules.
</script>

<form
  action="/search"
  method="GET"
  role="search"
  class="flex w-full max-w-md items-center gap-2"
>
  <label for="site-search" class="sr-only">Search anime</label>
  <input
    id="site-search"
    name="q"
    type="search"
    placeholder="Search anime..."
    autocomplete="off"
    class="w-full rounded-full border border-border-subtle bg-surface-hover px-4 py-2 text-sm text-ink placeholder:text-ink-faint focus:ring-2 focus:ring-accent focus:outline-none"
  />
</form>