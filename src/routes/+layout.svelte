<script lang="ts">
  import "../app.css";
  import NavBar from "$lib/components/NavBar.svelte";

  let { children } = $props();
</script>

<div class="min-h-screen bg-surface text-ink">
  <NavBar />

  <main class="mx-auto max-w-6xl px-4 py-6">
    {@render children()}
  </main>
</div>