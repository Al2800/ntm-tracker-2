<!--
  EmptyState.svelte
  Reusable empty state component with icon, title, description, and optional action.
  Use for: empty session list, no escalations, no timeline events, etc.
-->
<script lang="ts">
  export let icon: 'sessions' | 'escalations' | 'timeline' | 'search' | 'output' | 'generic' = 'generic';
  export let title = 'Nothing here yet';
  export let description = '';
  export let compact = false;

  const icons: Record<typeof icon, string> = {
    sessions: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 6h16M4 12h16M4 18h7" />`,
    escalations: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />`,
    timeline: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />`,
    search: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />`,
    output: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />`,
    generic: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />`
  };
</script>

<div
  class="flex flex-col items-center justify-center text-center"
  class:p-6={!compact}
  class:p-4={compact}
>
  <div
    class="flex items-center justify-center rounded-xl border border-dashed border-border bg-surface-base"
    class:h-16={!compact}
    class:w-16={!compact}
    class:h-12={compact}
    class:w-12={compact}
  >
    <svg
      class="text-text-subtle"
      class:h-8={!compact}
      class:w-8={!compact}
      class:h-6={compact}
      class:w-6={compact}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      {@html icons[icon]}
    </svg>
  </div>

  <h3
    class="mt-3 font-medium text-text-secondary"
    class:text-sm={!compact}
    class:text-xs={compact}
  >
    {title}
  </h3>

  {#if description}
    <p
      class="mt-1 max-w-xs text-text-muted"
      class:text-xs={!compact}
      class:text-2xs={compact}
    >
      {description}
    </p>
  {/if}

  {#if $$slots.action}
    <div class="mt-4">
      <slot name="action" />
    </div>
  {/if}
</div>
