<script lang="ts">
  import Modal from "$lib/components/Modal.svelte";

  let { open = false, onClose }: { open?: boolean; onClose?: () => void } =
    $props();
</script>

<Modal {open} onClose={onClose ?? (() => {})} label="Test dialog">
  <div class="p-4">
    <button type="button" data-testid="first-button">First</button>
    <button type="button" data-testid="last-button">Last</button>
  </div>
  {#snippet footer()}
    <div class="p-4">
      <button type="button" data-testid="footer-button">Footer</button>
    </div>
  {/snippet}
</Modal>