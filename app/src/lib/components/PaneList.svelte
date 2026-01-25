<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';
  import type { Pane } from '../types';
  import { getPaneStatus } from '../status';

  export let panes: Pane[] = [];
  export let dense = false;
  export let selectable = false;
  export let selectedPaneId: string | null = null;

  let listContainer: HTMLElement;

  const dispatch = createEventDispatcher<{ select: { paneUid: string } }>();

  const formatAge = (timestamp?: number) => {
    if (!timestamp) return null;
    const now = Date.now();
    const diffMs = Math.max(0, now - (timestamp < 1_000_000_000_000 ? timestamp * 1000 : timestamp));
    const minutes = Math.floor(diffMs / 60000);
    if (minutes < 1) return 'just now';
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ago`;
  };

  const handleKeydown = async (event: KeyboardEvent, index: number) => {
    if (!selectable || panes.length === 0) return;

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        await focusPane(Math.min(index + 1, panes.length - 1));
        break;
      case 'ArrowUp':
        event.preventDefault();
        await focusPane(Math.max(index - 1, 0));
        break;
      case 'Home':
        event.preventDefault();
        await focusPane(0);
        break;
      case 'End':
        event.preventDefault();
        await focusPane(panes.length - 1);
        break;
    }
  };

  const focusPane = async (index: number) => {
    await tick();
    const buttons = listContainer?.querySelectorAll('[data-pane-button]');
    const button = buttons?.[index] as HTMLElement;
    button?.focus();
  };
</script>

<div
  bind:this={listContainer}
  class={dense ? 'mt-3 space-y-1.5' : 'mt-3 space-y-2'}
  role={selectable ? 'listbox' : 'list'}
  aria-label="Pane list"
>
  {#each panes as pane, index (pane.paneUid)}
    {@const paneStatus = getPaneStatus(pane.status)}
    <button
      data-pane-button
      type="button"
      role={selectable ? 'option' : 'listitem'}
      aria-selected={selectable ? selectedPaneId === pane.paneUid : undefined}
      aria-label="Pane {pane.index}, {paneStatus.label}{pane.agentType ? `, ${pane.agentType}` : ''}"
      class={`flex w-full items-center justify-between gap-3 rounded-lg border px-3 py-2 text-left transition focus:outline-none focus-visible:ring-2 focus-visible:ring-border-focus focus-visible:ring-offset-1 focus-visible:ring-offset-surface-base ${
        selectedPaneId === pane.paneUid
          ? 'card-selected'
          : 'border-border bg-surface-base hover:border-border-strong'
      } ${selectable ? 'card-interactive' : 'cursor-default'}`}
      on:click={() => selectable && dispatch('select', { paneUid: pane.paneUid })}
      on:keydown={(e) => handleKeydown(e, index)}
    >
      <div class="flex items-center gap-3">
        <span class="status-dot {paneStatus.dot}"></span>
        <div>
          <p class="text-sm font-semibold text-text-primary">Pane {pane.index}</p>
          <div class="flex flex-wrap items-center gap-2 text-[11px] uppercase tracking-label-tight text-text-subtle">
            {#if pane.agentType}
              <span>{pane.agentType}</span>
              <span class="text-text-subtle/50">•</span>
            {/if}
            <span class="font-mono normal-case text-text-muted">{pane.paneUid.slice(0, 8)}</span>
          </div>
        </div>
      </div>
      <div class="flex flex-col items-end gap-1 text-xs">
        <span class="badge {paneStatus.badge} px-2 py-0.5">
          {paneStatus.label}
        </span>
        {#if pane.currentCommand}
          <span class="hidden sm:inline text-text-muted">cmd: {pane.currentCommand}</span>
        {/if}
        {#if pane.lastActivityAt}
          <span class="text-[11px] text-text-subtle">Active {formatAge(pane.lastActivityAt)}</span>
        {/if}
      </div>
    </button>
  {/each}

  {#if panes.length === 0}
    <p class="text-sm text-text-subtle">No panes reported.</p>
  {/if}
</div>
