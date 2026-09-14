<script lang="ts">
  /**
   * A single pulsing placeholder block.
   *
   * The one primitive every skeleton is composed from. Sizing and shaping are
   * left to the caller's `class`, so the same component covers a poster, a
   * text line, or a chip without a prop per shape.
   */
  let { class: className = "" }: { class?: string } = $props();
</script>

<div class="animate-pulse rounded bg-surface-hover {className}" aria-hidden="true"></div>