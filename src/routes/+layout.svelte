<script lang="ts">
  import "../app.css";

  let { children } = $props();
</script>

<div class="min-h-screen bg-surface text-ink">
  {@render children()}
</div>