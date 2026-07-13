<script>
  let { title = "", children } = $props();
</script>

<section class="card">
  {#if title}<h3>{title}</h3>{/if}
  {@render children?.()}
</section>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h3 {
    font-size: 13px;
    font-weight: 600;
    color: var(--muted);
    text-transform: none;
    letter-spacing: 0.02em;
  }
</style>
