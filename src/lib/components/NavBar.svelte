<script lang="ts">
  import SearchBox from "./SearchBox.svelte";
  import SettingsDrawer from "./SettingsDrawer.svelte";
  import { page } from "$app/state"; // Or `$app/stores` depending on SvelteKit version

  /** Whether the settings drawer is open. */
  let settingsOpen = $state(false);

  /** Helper to mark active link states based on route path */
  const isActive = (path: string) => page.url.pathname === path;
</script>

<header
  class="sticky top-0 z-40 border-b border-border-subtle/80 bg-surface-raised/80 backdrop-blur-md transition-all"
>
  <nav class="mx-auto flex max-w-7xl items-center justify-between gap-4 px-4 py-2.5 sm:px-6">
    <!-- Brand / Logo -->
    <a
      href="/"
      class="group flex items-center gap-2.5 shrink-0 text-lg font-bold tracking-tight text-ink transition-opacity hover:opacity-90 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent rounded-lg p-1"
    >
      <!-- Optional Logo Glyph Accent -->
      <div class="flex h-8 w-8 items-center justify-center rounded-xl bg-accent text-white shadow-sm transition-transform group-hover:scale-105">
        <svg class="h-5 w-5 fill-current" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
        </svg>
      </div>
      <span class="bg-linear-to-r from-ink to-ink-muted bg-clip-text text-transparent">Kitsune</span>
    </a>

    <!-- Center Search Container -->
    <div class="flex-1 max-w-md mx-2">
      <SearchBox />
    </div>

    <!-- Navigation Links & Profile Action -->
    <div class="flex items-center gap-1.5 sm:gap-2">
      <!-- Nav Pill Links -->
      <div class="flex items-center gap-1 rounded-full border border-border-subtle bg-surface-base/50 p-1 shadow-inner">
        <a
          href="/list"
          aria-current={isActive("/list") ? "page" : undefined}
          class="flex items-center gap-1.5 rounded-full px-3 py-1.5 text-xs font-semibold transition-all focus:outline-none focus-visible:ring-2 focus-visible:ring-accent
            {isActive('/list')
              ? 'bg-accent text-white shadow-sm'
              : 'text-ink-muted hover:text-ink hover:bg-surface-hover'}"
        >
          <!-- Bookmark Icon -->
          <svg class="h-3.5 w-3.5 fill-none stroke-current stroke-2" viewBox="0 0 24 24" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"/>
          </svg>
          <span class="hidden sm:inline">My List</span>
        </a>

        <a
          href="/filter"
          aria-current={isActive("/filter") ? "page" : undefined}
          class="flex items-center gap-1.5 rounded-full px-3 py-1.5 text-xs font-semibold transition-all focus:outline-none focus-visible:ring-2 focus-visible:ring-accent
            {isActive('/filter')
              ? 'bg-accent text-white shadow-sm'
              : 'text-ink-muted hover:text-ink hover:bg-surface-hover'}"
        >
          <!-- Sliders Icon -->
          <svg class="h-3.5 w-3.5 fill-none stroke-current stroke-2" viewBox="0 0 24 24" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4"/>
          </svg>
          <span class="hidden sm:inline">Filter</span>
        </a>
      </div>

      <!-- Vertical Divider -->
      <div class="h-5 w-px bg-border-subtle mx-1" aria-hidden="true"></div>

      <!-- Settings / Profile Button -->
      <button
        type="button"
        onclick={() => (settingsOpen = true)}
        aria-label="Settings"
        title="Settings"
        data-testid="settings-open"
        class="relative flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-border-subtle bg-surface-raised text-ink-muted transition-all hover:border-accent hover:text-accent hover:shadow-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-accent active:scale-95"
      >
        <svg
          class="h-4 w-4 fill-none stroke-current stroke-2 transition-transform duration-300 hover:rotate-45"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
          />
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
          />
        </svg>
      </button>
    </div>
  </nav>
</header>

<SettingsDrawer open={settingsOpen} onClose={() => (settingsOpen = false)} />