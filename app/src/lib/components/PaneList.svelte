<script lang="ts">
  import type { Pane } from '../types';

  export let panes: Pane[] = [];

  const statusClasses: Record<Pane['status'], string> = {
    active: 'bg-emerald-500/20 text-emerald-200',
    idle: 'bg-slate-500/20 text-slate-200',
    waiting: 'bg-amber-500/20 text-amber-200',
    ended: 'bg-rose-500/20 text-rose-200',
    unknown: 'bg-slate-700/40 text-slate-300'
  };
</script>

<div class="mt-3 space-y-2">
  {#each panes as pane (pane.paneUid)}
    <div class="flex items-center justify-between rounded-lg border border-slate-800 bg-slate-950/60 px-3 py-2">
      <div class="flex items-center gap-3">
        <span class="text-sm font-semibold text-slate-200">Pane {pane.index}</span>
        {#if pane.agentType}
          <span class="text-xs uppercase tracking-[0.2em] text-slate-400">{pane.agentType}</span>
        {/if}
      </div>
      <div class="flex items-center gap-3 text-xs text-slate-400">
        <span class={`rounded-full px-2 py-0.5 ${statusClasses[pane.status]}`}>
          {pane.status}
        </span>
        {#if pane.currentCommand}
          <span class="hidden sm:inline">cmd: {pane.currentCommand}</span>
        {/if}
      </div>
    </div>
  {/each}

  {#if panes.length === 0}
    <p class="text-sm text-slate-500">No panes reported.</p>
  {/if}
</div>
