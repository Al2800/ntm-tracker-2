<!--
  LoadingSkeleton.svelte
  Skeleton loader for content areas.
  Use for: loading session list, loading output preview, etc.
-->
<script lang="ts">
  export let variant: 'card' | 'list-item' | 'text' | 'chart' = 'card';
  export let count = 1;
  export let compact = false;
</script>

{#if variant === 'card'}
  {#each Array(count) as _, i (i)}
    <div
      class="animate-pulse rounded-card border border-border bg-surface-raised"
      class:p-4={!compact}
      class:p-3={compact}
    >
      <div class="flex items-start gap-3">
        <div
          class="rounded-lg bg-surface-base"
          class:h-10={!compact}
          class:w-10={!compact}
          class:h-8={compact}
          class:w-8={compact}
        ></div>
        <div class="flex-1 space-y-2">
          <div class="h-4 w-1/2 rounded bg-surface-base"></div>
          <div class="h-3 w-3/4 rounded bg-surface-base"></div>
        </div>
      </div>
      <div class="mt-3 grid grid-cols-3 gap-2">
        <div class="h-8 rounded bg-surface-base"></div>
        <div class="h-8 rounded bg-surface-base"></div>
        <div class="h-8 rounded bg-surface-base"></div>
      </div>
    </div>
  {/each}
{:else if variant === 'list-item'}
  <div class="space-y-1.5">
    {#each Array(count) as _, i (i)}
      <div
        class="animate-pulse rounded-lg border border-border bg-surface-raised"
        class:p-3={!compact}
        class:p-2={compact}
      >
        <div class="flex items-center gap-2.5">
          <div
            class="rounded-md bg-surface-base"
            class:h-5={!compact}
            class:w-5={!compact}
            class:h-4={compact}
            class:w-4={compact}
          ></div>
          <div class="flex-1 space-y-1.5">
            <div class="h-3.5 w-1/3 rounded bg-surface-base"></div>
            <div class="h-2.5 w-1/2 rounded bg-surface-base"></div>
          </div>
          <div class="h-3 w-8 rounded bg-surface-base"></div>
        </div>
      </div>
    {/each}
  </div>
{:else if variant === 'text'}
  <div class="animate-pulse space-y-2">
    {#each Array(count) as _, i (i)}
      <div class="h-3 rounded bg-surface-base" style="width: {75 + Math.random() * 25}%"></div>
    {/each}
  </div>
{:else if variant === 'chart'}
  <div class="animate-pulse rounded-card border border-border bg-surface-raised p-4">
    <div class="mb-4 flex items-center justify-between">
      <div class="h-4 w-24 rounded bg-surface-base"></div>
      <div class="flex gap-2">
        <div class="h-6 w-16 rounded-full bg-surface-base"></div>
        <div class="h-6 w-16 rounded-full bg-surface-base"></div>
      </div>
    </div>
    <div class="flex items-end justify-between gap-1" style="height: 100px">
      {#each Array(24) as _, i (i)}
        <div
          class="flex-1 rounded-sm bg-surface-base"
          style="height: {20 + Math.random() * 80}%"
        ></div>
      {/each}
    </div>
  </div>
{/if}
