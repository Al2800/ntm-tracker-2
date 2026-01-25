<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { events } from '../stores/events';
  import type { TrackerEvent } from '../types';

  const dispatch = createEventDispatcher<{
    focus: { eventId: number };
    dismiss: { eventId: number };
    snooze: { eventId: number };
  }>();

  const formatAge = (timestamp: number) => {
    const now = Date.now();
    const diffMs = Math.max(0, now - timestamp * (timestamp < 1_000_000_000_000 ? 1000 : 1));
    const minutes = Math.floor(diffMs / 60000);
    if (minutes < 1) return 'just now';
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ago`;
  };

  $: pending = $events.filter(
    (event) => event.type === 'escalation' && (event.status ?? 'pending') === 'pending'
  );

  const labelFor = (event: TrackerEvent) => `${event.sessionUid}:${event.paneUid}`;
</script>

<div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5">
  <div class="flex items-center justify-between">
    <div>
      <h3 class="text-sm font-semibold uppercase tracking-[0.2em] text-slate-300">
        Escalation Inbox
      </h3>
      <p class="mt-1 text-xs text-slate-500">Pending alerts that need human attention.</p>
    </div>
    <span class="rounded-full border border-rose-500/40 bg-rose-500/10 px-3 py-1 text-xs text-rose-200">
      {pending.length} open
    </span>
  </div>

  {#if pending.length === 0}
    <div class="mt-4 rounded-xl border border-dashed border-slate-800 bg-slate-950/60 p-6 text-center text-sm text-slate-500">
      Inbox clear. Escalations will appear here as soon as they trigger.
    </div>
  {:else}
    <div class="mt-4 space-y-3">
      {#each pending as event (event.id)}
        <div class="rounded-xl border border-slate-800/80 bg-slate-950/70 p-4">
          <div class="flex items-start justify-between gap-4">
            <div>
              <p class="text-sm font-semibold text-slate-100">⚠ {labelFor(event)}</p>
              <p class="mt-1 text-xs text-slate-400">{event.message ?? 'Needs attention.'}</p>
            </div>
            <span class="text-xs text-slate-500">{formatAge(event.detectedAt)}</span>
          </div>
          <div class="mt-3 flex flex-wrap gap-2 text-xs">
            <button
              class="rounded-lg border border-slate-700/80 px-3 py-1.5 text-slate-200 transition hover:border-slate-500"
              on:click={() => dispatch('focus', { eventId: event.id })}
            >
              Focus
            </button>
            <button
              class="rounded-lg border border-slate-700/80 px-3 py-1.5 text-slate-200 transition hover:border-slate-500"
              on:click={() => dispatch('snooze', { eventId: event.id })}
            >
              Snooze 15m
            </button>
            <button
              class="rounded-lg border border-slate-700/80 px-3 py-1.5 text-slate-200 transition hover:border-slate-500"
              on:click={() => dispatch('dismiss', { eventId: event.id })}
            >
              Dismiss
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
