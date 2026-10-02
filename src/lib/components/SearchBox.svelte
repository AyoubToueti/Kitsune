<script lang="ts">
  let query = $state("");

  function clearQuery() {
    query = "";
  }
</script>

<form
  action="/search"
  method="GET"
  role="search"
  class="relative flex w-full max-w-md items-center"
>
  <label for="site-search" class="sr-only">Search anime</label>

  <!-- Magnifying Glass Icon (Left) -->
  <div
    class="pointer-events-none absolute left-3.5 flex items-center text-ink-muted"
    aria-hidden="true"
  >
    <svg
      class="h-4 w-4 fill-none stroke-current stroke-2 transition-colors group-focus-within:text-accent"
      viewBox="0 0 24 24"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
      />
    </svg>
  </div>

  <!-- Search Input -->
  <input
    id="site-search"
    name="q"
    type="search"
    bind:value={query}
    placeholder="Search anime..."
    autocomplete="off"
    class="h-9 w-full rounded-full border border-border-subtle bg-surface-raised pl-10 pr-10 text-xs font-medium text-ink placeholder:text-ink-faint shadow-inner transition-all hover:border-border focus:border-accent focus:bg-surface-base focus:outline-none focus:ring-2 focus:ring-accent/20"
  />

  <!-- Right Actions: Clear Button or Keyboard Hint -->
  <div class="absolute right-3 flex items-center gap-1">
    {#if query.length > 0}
      <!-- Clear Button -->
      <button
        type="button"
        onclick={clearQuery}
        aria-label="Clear search input"
        class="flex h-4 w-4 items-center justify-center rounded-full text-ink-muted transition-colors hover:bg-surface-hover hover:text-ink focus:outline-none"
      >
        <svg
          class="h-3 w-3 fill-none stroke-current stroke-2"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M6 18L18 6M6 6l12 12"
          />
        </svg>
      </button>
    {:else}
      <!-- Shortcut Badge -->
      <kbd
        class="hidden select-none rounded border border-border-subtle bg-surface-hover px-1.5 py-0.5 text-[10px] font-semibold text-ink-faint sm:inline-block"
      >
        ⌘K
      </kbd>
    {/if}
  </div>
</form>