<script lang="ts">
  import SearchBox from "./SearchBox.svelte";
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
    </div>
  </nav>
</header>