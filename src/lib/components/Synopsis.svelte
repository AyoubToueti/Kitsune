<script lang="ts">
  import { stripHtml } from "$lib/text";

  let { text }: { text?: string } = $props();

  const clean = $derived(stripHtml(text));
</script>

{#if clean}
  <p class="max-w-3xl text-sm leading-relaxed text-ink-muted">{clean}</p>
{/if}