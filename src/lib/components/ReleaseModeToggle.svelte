<script lang="ts">
  import type { SearchMode } from "$lib/types";

  let {
    mode = "episodes",
    disabled = false,
    onChange,
  }: {
    /** The mode currently selected. */
    mode?: SearchMode;
    /** Whether the control is disabled (e.g. during searches). */
    disabled?: boolean;
    /** Called with the mode whose segment was clicked. */
    onChange?: (mode: SearchMode) => void;
  } = $props();

  /** The two segments, in display order. */
  const SEGMENTS: { value: SearchMode; label: string }[] = [
    { value: "episodes", label: "Episodes" },
    { value: "packs", label: "Packs" },
  ];
</script>

<div
  data-testid="release-mode-toggle"
  role="group"
  aria-label="Release type"
  class="relative inline-flex items-center rounded-full border border-border-subtle bg-surface p-0.5 {disabled ? 'opacity-60' : ''}"
>
  <!-- Animated sliding pill background -->
  <div
    class="absolute top-0.5 bottom-0.5 w-[calc(50%-2px)] rounded-full bg-accent transition-transform duration-300 ease-out"
    style="transform: translateX({mode === 'packs' ? '100%' : '0%'});"
    aria-hidden="true"
  ></div>

  {#each SEGMENTS as segment (segment.value)}
    <button
      type="button"
      disabled={disabled}
      onclick={() => onChange?.(segment.value)}
      aria-pressed={segment.value === mode}
      data-mode={segment.value}
      class="relative z-10 w-20 rounded-full px-3 py-1 text-center text-xs font-medium transition-colors duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed {segment.value ===
      mode
        ? 'text-white'
        : 'text-ink-muted hover:text-ink'}"
    >
      {segment.label}
    </button>
  {/each}
</div>