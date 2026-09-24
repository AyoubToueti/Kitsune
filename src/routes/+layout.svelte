<script lang="ts">
  import "../app.css";
  import NavBar from "$lib/components/NavBar.svelte";
  import ResumeDisc from "$lib/components/ResumeDisc.svelte";

  let { children } = $props();
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
</div>