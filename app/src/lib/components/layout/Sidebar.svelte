<!--
  Sidebar.svelte
  Collapsible sidebar wrapper with header and content slots.
  See docs/information-architecture.md for design rationale.
-->
<script lang="ts">
  export let title = 'Sessions';
  export let subtitle = '';
  export let count: number | null = null;
</script>

<div class="flex h-full flex-col">
  <!-- Sidebar header -->
  <div class="flex items-center justify-between border-b border-border px-4 py-3">
    <div>
      <h2 class="label">{title}</h2>
      {#if subtitle}
        <p class="mt-0.5 text-2xs text-text-subtle">{subtitle}</p>
      {/if}
    </div>
    {#if count !== null}
      <span class="badge badge-neutral text-2xs">{count}</span>
    {/if}
  </div>

  <!-- Filter bar slot -->
  {#if $$slots.filters}
    <div class="border-b border-border px-4 py-2">
      <slot name="filters" />
    </div>
  {/if}

  <!-- Main content -->
  <div class="flex-1 overflow-y-auto px-3 py-3">
    <slot />
  </div>

  <!-- Actions slot -->
  {#if $$slots.actions}
    <div class="border-t border-border px-4 py-3">
      <slot name="actions" />
    </div>
  {/if}
</div>
