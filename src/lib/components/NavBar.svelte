<script lang="ts">
  import SearchBox from "./SearchBox.svelte";
  import SettingsDrawer from "./SettingsDrawer.svelte";

  /** Whether the settings drawer is open. */
  let settingsOpen = $state(false);
</script>

<header
  class="sticky top-0 z-10 border-b border-border-subtle bg-surface-raised/95 backdrop-blur"
>
  <nav class="mx-auto flex max-w-6xl items-center justify-between gap-4 px-4 py-3">
    <a
      href="/"
      class="shrink-0 text-lg font-semibold tracking-tight text-ink transition-colors hover:text-accent"
    >
      Kitsune
    </a>

    <div class="flex min-w-0 flex-1 items-center justify-end gap-3">
      <SearchBox />

      <a
        href="/list"
        class="shrink-0 rounded-full border border-border-subtle bg-surface-hover px-4 py-2 text-sm text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        My List
      </a>

      <a
        href="/filter"
        class="shrink-0 rounded-full border border-border-subtle bg-surface-hover px-4 py-2 text-sm text-ink transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        Filter
      </a>

      <!-- Profile / settings. An avatar disc rather than a word, so the row
           stays compact; the label is on the button for screen readers. -->
      <button
        type="button"
        onclick={() => (settingsOpen = true)}
        aria-label="Settings"
        title="Settings"
        data-testid="settings-open"
        class="grid size-9 shrink-0 place-items-center rounded-full border border-border-subtle bg-surface-hover text-ink-muted transition-colors hover:border-accent hover:text-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          class="size-4"
          aria-hidden="true"
        >
          <path
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 15a3 3 0 100-6 3 3 0 000 6z"
          />
          <path
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 11-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 11-4 0v-.09a1.65 1.65 0 00-1-1.51 1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 11-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 110-4h.09a1.65 1.65 0 001.51-1 1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 112.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 114 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 112.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 110 4h-.09a1.65 1.65 0 00-1.51 1z"
          />
        </svg>
      </button>
    </div>
  </nav>
</header>

<SettingsDrawer open={settingsOpen} onClose={() => (settingsOpen = false)} />