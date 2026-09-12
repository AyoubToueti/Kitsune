<script lang="ts">
  import SearchBox from "./SearchBox.svelte";
</script>

<header
  class="sticky top-0 z-10 border-b border-border-subtle bg-surface-raised/95 backdrop-blur"
>
  <nav class="mx-auto flex max-w-6xl items-center justify-between gap-4 px-4 py-3">
    <a
      href="/"
      class="text-lg font-semibold tracking-tight text-ink transition-colors hover:text-accent"
    >
      Kitsune
    </a>

    <SearchBox />
  </nav>
</header>