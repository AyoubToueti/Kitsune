// A Svelte action that moves its element to `document.body`.
//
// A `position: fixed` overlay is only truly viewport-fixed while no ancestor
// creates a stacking context. `position: sticky`, `transform`, `filter` and
// `will-change` all do -- and the detail page's sidebar is `lg:sticky`, so a
// modal rendered inside it is trapped in that context and paints UNDER later
// siblings (the Recommended row) regardless of its `z-index`.
//
// Moving the node to `<body>` removes every such ancestor, so the overlay's
// `z-index` competes at the root. Used by `Modal.svelte` so every dialog
// escapes any caller's stacking context without the caller knowing.
//
// The action owns both directions: it appends on mount and removes on destroy,
// so Svelte's `{#if}` teardown does not leave an orphan node on `<body>`.

export function portal(node: HTMLElement): { destroy: () => void } {
  document.body.appendChild(node);

  return {
    destroy() {
      node.remove();
    },
  };
}