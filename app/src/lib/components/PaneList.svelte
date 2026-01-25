<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Pane } from '../types';

  export let panes: Pane[] = [];
  export let dense = false;
  export let selectable = false;
  export let selectedPaneId: string | null = null;

  const dispatch = createEventDispatcher<{ select: { paneUid: string } }>();

  const statusClasses: Record<Pane['status'], string> = {
    active: 'bg-emerald-500/15 text-emerald-200 ring-1 ring-emerald-400/40',
    idle: 'bg-slate-500/15 text-slate-200 ring-1 ring-slate-500/40',
    waiting: 'bg-amber-500/15 text-amber-200 ring-1 ring-amber-400/40',
    ended: 'bg-rose-500/15 text-rose-200 ring-1 ring-rose-400/40',
    unknown: 'bg-slate-700/40 text-slate-300 ring-1 ring-slate-600/40'
  };

  const statusDot: Record<Pane['status'], string> = {
    active: 'bg-emerald-400',
    idle: 'bg-slate-300',
    waiting: 'bg-amber-400',
    ended: 'bg-rose-400',
    unknown: 'bg-slate-500'
  };

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
</script>

<div class={dense ? 'mt-3 space-y-1.5' : 'mt-3 space-y-2'}>
  {#each panes as pane (pane.paneUid)}
    <button
      type="button"
      class={`flex w-full items-center justify-between gap-3 rounded-lg border px-3 py-2 text-left transition ${
        selectedPaneId === pane.paneUid
          ? 'border-sky-500/60 bg-sky-500/10'
          : 'border-slate-800 bg-slate-950/60 hover:border-slate-700/80'
      } ${selectable ? '' : 'cursor-default'}`}
      on:click={() => selectable && dispatch('select', { paneUid: pane.paneUid })}
    >
      <div class="flex items-center gap-3">
        <span class={`h-2 w-2 rounded-full ${statusDot[pane.status]}`}></span>
        <div>
          <p class="text-sm font-semibold text-slate-200">Pane {pane.index}</p>
          <div class="flex flex-wrap items-center gap-2 text-[11px] uppercase tracking-[0.2em] text-slate-500">
            {#if pane.agentType}
              <span>{pane.agentType}</span>
              <span class="text-slate-700">•</span>
            {/if}
            <span class="font-mono normal-case text-slate-400">{pane.paneUid.slice(0, 8)}</span>
          </div>
        </div>
      </div>
      <div class="flex flex-col items-end gap-1 text-xs text-slate-400">
        <span class={`rounded-full px-2 py-0.5 ${statusClasses[pane.status]}`}>
          {pane.status}
        </span>
        {#if pane.currentCommand}
          <span class="hidden sm:inline">cmd: {pane.currentCommand}</span>
        {/if}
        {#if pane.lastActivityAt}
          <span class="text-[11px] text-slate-500">Active {formatAge(pane.lastActivityAt)}</span>
        {/if}
      </div>
    </button>
  {/each}

  {#if panes.length === 0}
    <p class="text-sm text-slate-500">No panes reported.</p>
  {/if}
</div>
