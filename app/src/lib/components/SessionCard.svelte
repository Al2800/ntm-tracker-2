<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Session } from '../types';

  export let session: Session;
  export let expanded = false;

  const dispatch = createEventDispatcher<{ toggle: { sessionUid: string } }>();

  const statusClasses: Record<Session['status'], string> = {
    active: 'bg-emerald-500/20 text-emerald-200',
    idle: 'bg-slate-500/20 text-slate-200',
    ended: 'bg-rose-500/20 text-rose-200',
    unknown: 'bg-slate-700/40 text-slate-300'
  };
</script>

<div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
  <button
    type="button"
    class="flex w-full items-center justify-between text-left"
    on:click={() => dispatch('toggle', { sessionUid: session.sessionUid })}
  >
    <div>
      <p class="text-lg font-semibold text-white">{session.name}</p>
      <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Session</p>
    </div>
    <div class="flex items-center gap-3 text-xs text-slate-400">
      <span class={`rounded-full px-2 py-0.5 ${statusClasses[session.status]}`}>
        {session.status}
      </span>
      <span>{session.paneCount} panes</span>
      <span class="text-lg">{expanded ? '▾' : '▸'}</span>
    </div>
  </button>

  {#if expanded}
    <div class="mt-4">
      <slot />
    </div>
  {/if}
</div>
