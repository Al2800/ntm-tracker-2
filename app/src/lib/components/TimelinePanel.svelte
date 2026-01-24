<script lang="ts">
  import { events } from '../stores/events';
  import { selectedSession } from '../stores/sessions';
  import type { TrackerEvent } from '../types';

  let order: 'desc' | 'asc' = 'desc';

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp < 1_000_000_000_000 ? timestamp * 1000 : timestamp);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  $: sessionUid = $selectedSession?.sessionUid;
  $: filtered = $events.filter((event) => (sessionUid ? event.sessionUid === sessionUid : true));
  $: sorted = [...filtered].sort((a, b) =>
    order === 'desc' ? b.detectedAt - a.detectedAt : a.detectedAt - b.detectedAt
  );

  const labelFor = (event: TrackerEvent) => `${event.sessionUid}:${event.paneUid}`;
</script>

<div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-semibold uppercase tracking-[0.2em] text-slate-300">Timeline</h3>
    <button
      type="button"
      class="text-xs text-slate-400 hover:text-slate-200"
      on:click={() => (order = order === 'desc' ? 'asc' : 'desc')}
    >
      {order === 'desc' ? 'Newest first' : 'Oldest first'}
    </button>
  </div>

  {#if sorted.length === 0}
    <div class="mt-4 rounded-lg border border-dashed border-slate-700 p-6 text-center text-sm text-slate-500">
      No events yet.
    </div>
  {:else}
    <div class="mt-4 space-y-2 text-sm text-slate-300">
      {#each sorted as event (event.id)}
        <div class="flex items-center justify-between rounded-lg border border-slate-800 bg-slate-950/60 px-3 py-2">
          <div class="flex items-center gap-3">
            <span class="text-xs text-slate-500">{formatTime(event.detectedAt)}</span>
            <span class="font-semibold text-slate-100">{event.type}</span>
            <span class="text-xs text-slate-400">{labelFor(event)}</span>
          </div>
          <span class="text-xs text-slate-400">{event.message ?? ''}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>
