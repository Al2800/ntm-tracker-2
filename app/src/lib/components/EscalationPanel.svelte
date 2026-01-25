<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { events } from '../stores/events';
  import { getEscalationSeverity, ESCALATION_SEVERITY, type EscalationSeverity } from '../status';
  import type { TrackerEvent } from '../types';
  import EmptyState from './states/EmptyState.svelte';

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

  // Map TrackerEvent severity to EscalationSeverity
  const mapSeverity = (severity?: 'info' | 'warn' | 'error'): EscalationSeverity => {
    if (severity === 'error') return 'high';
    if (severity === 'warn') return 'medium';
    return 'low';
  };

  $: pending = $events.filter(
    (event) => event.eventType === 'escalation' && (event.status ?? 'pending') === 'pending'
  );
  $: sorted = [...pending].sort((a, b) => {
    const rankA = ESCALATION_SEVERITY[mapSeverity(a.severity)]?.rank ?? 4;
    const rankB = ESCALATION_SEVERITY[mapSeverity(b.severity)]?.rank ?? 4;
    return rankA - rankB;
  });

  const labelFor = (event: TrackerEvent) => {
    const parts = [];
    if (event.sessionId) parts.push(event.sessionId.slice(0, 8));
    if (event.paneId) parts.push(event.paneId.slice(0, 6));
    return parts.join(':') || 'Unknown';
  };
</script>

<div class="card">
  <div class="flex items-center justify-between">
    <div>
      <h3 class="label">Escalation Inbox</h3>
      <p class="mt-1 text-xs text-text-subtle">Pending alerts that need human attention.</p>
    </div>
    <span
      class="badge {pending.length > 0 ? 'badge-error' : 'badge-neutral'}"
      aria-label="{pending.length} open escalations"
    >
      {pending.length} open
    </span>
  </div>

  {#if sorted.length === 0}
    <div class="mt-4">
      <EmptyState
        icon="escalations"
        title="Inbox clear"
        description="Escalations will appear here as they trigger."
        compact
      />
    </div>
  {:else}
    <div class="mt-4 space-y-3" role="list" aria-label="Pending escalations">
      {#each sorted as event (event.id)}
        {@const severity = getEscalationSeverity(mapSeverity(event.severity))}
        <div class="card-compact card-interactive {severity.rank <= 1 ? 'card-critical' : ''}" role="listitem">
          <div class="flex items-start justify-between gap-4">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="status-dot {severity.dot}" aria-hidden="true"></span>
                <span class="badge {severity.badge} py-0.5 px-2 text-[10px]">
                  <span class="sr-only">Severity:</span> {severity.label}
                </span>
                <span class="text-xs font-mono text-text-muted truncate">{labelFor(event)}</span>
              </div>
              <p class="mt-2 text-sm text-text-primary">{event.message ?? 'Needs attention.'}</p>
            </div>
            <span class="text-xs text-text-subtle shrink-0">{formatAge(event.detectedAt)}</span>
          </div>
          <div class="mt-3 flex flex-wrap gap-2" role="group" aria-label="Escalation actions">
            <button
              class="btn btn-sm btn-secondary"
              on:click={() => dispatch('focus', { eventId: event.id })}
              aria-label="Focus on escalation from {labelFor(event)}"
            >
              Focus
            </button>
            <button
              class="btn btn-sm btn-ghost"
              on:click={() => dispatch('snooze', { eventId: event.id })}
              aria-label="Snooze escalation from {labelFor(event)} for 15 minutes"
            >
              Snooze 15m
            </button>
            <button
              class="btn btn-sm btn-ghost"
              on:click={() => dispatch('dismiss', { eventId: event.id })}
              aria-label="Dismiss escalation from {labelFor(event)}"
            >
              Dismiss
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
