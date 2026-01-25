<script lang="ts">
  import { events } from '../stores/events';
  import { selectedSession } from '../stores/sessions';
  import { getEventType } from '../status';
  import type { TrackerEvent } from '../types';

  export let limit = 20;

  let order: 'desc' | 'asc' = 'desc';

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp < 1_000_000_000_000 ? timestamp * 1000 : timestamp);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp < 1_000_000_000_000 ? timestamp * 1000 : timestamp);
    const today = new Date();
    if (date.toDateString() === today.toDateString()) return 'Today';
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);
    if (date.toDateString() === yesterday.toDateString()) return 'Yesterday';
    return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
  };

  $: sessionUid = $selectedSession?.sessionUid;
  $: filtered = $events.filter((event) => (sessionUid ? event.sessionUid === sessionUid : true));
  $: sorted = [...filtered]
    .sort((a, b) => order === 'desc' ? b.detectedAt - a.detectedAt : a.detectedAt - b.detectedAt)
    .slice(0, limit);
  $: hasMore = filtered.length > limit;

  const labelFor = (event: TrackerEvent) => {
    const parts = [];
    if (event.sessionUid) parts.push(event.sessionUid.slice(0, 6));
    if (event.paneUid) parts.push(event.paneUid.slice(0, 4));
    return parts.join(':') || '';
  };
</script>

<div class="card">
  <div class="flex items-center justify-between">
    <div>
      <h3 class="label">Timeline</h3>
      <p class="mt-1 text-xs text-text-subtle">
        {#if sessionUid}
          Events for selected session
        {:else}
          Recent session, pane, and escalation events
        {/if}
      </p>
    </div>
    <button
      type="button"
      class="chip chip-default"
      on:click={() => (order = order === 'desc' ? 'asc' : 'desc')}
    >
      {order === 'desc' ? 'Newest first' : 'Oldest first'}
    </button>
  </div>

  {#if sorted.length === 0}
    <div class="mt-4 rounded-xl border border-dashed border-border bg-surface-base p-6 text-center">
      <p class="text-sm text-text-subtle">No events yet</p>
      <p class="mt-1 text-xs text-text-muted">Events will appear as sessions run.</p>
    </div>
  {:else}
    <div class="mt-4 space-y-1.5 max-h-[320px] overflow-y-auto">
      {#each sorted as event (event.id)}
        {@const eventType = getEventType(event.type)}
        <div class="tray-item group">
          <div class="flex items-center gap-3 min-w-0">
            <span class="text-base" title={eventType.description}>{eventType.icon}</span>
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="badge {eventType.badge} py-0 px-1.5 text-[10px]">
                  {eventType.label}
                </span>
                {#if labelFor(event)}
                  <span class="text-[10px] font-mono text-text-muted truncate">{labelFor(event)}</span>
                {/if}
              </div>
              {#if event.message}
                <p class="mt-0.5 text-xs text-text-secondary truncate">{event.message}</p>
              {/if}
            </div>
          </div>
          <div class="text-right shrink-0">
            <p class="text-[10px] text-text-muted">{formatDate(event.detectedAt)}</p>
            <p class="text-[10px] text-text-subtle">{formatTime(event.detectedAt)}</p>
          </div>
        </div>
      {/each}
      {#if hasMore}
        <p class="text-center text-[10px] text-text-muted pt-2">
          +{filtered.length - limit} more events
        </p>
      {/if}
    </div>
  {/if}
</div>
