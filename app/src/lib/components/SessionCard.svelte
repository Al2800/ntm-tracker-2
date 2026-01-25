<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Session } from '../types';
  import { getSessionStatus } from '../status';

  export let session: Session;
  export let expanded = false;
  export let dense = false;
  export let pinned = false;

  const dispatch = createEventDispatcher<{
    toggle: { sessionId: string };
    pin: { sessionId: string };
  }>();

  const handlePin = (e: Event) => {
    e.stopPropagation();
    dispatch('pin', { sessionId: session.sessionId });
  };

  // Use centralized status system
  $: sessionStatus = getSessionStatus(session.status);

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

  $: panes = session.panes ?? [];
  $: activeCount = panes.filter((pane) => pane.status === 'active').length;
  $: idleCount = panes.filter((pane) => pane.status === 'idle').length;
  $: waitingCount = panes.filter((pane) => pane.status === 'waiting').length;
  $: lastSeen = formatAge(session.lastSeenAt);
</script>

<div
  data-session-card
  id="session-{session.sessionId}"
  tabindex="0"
  role="option"
  aria-selected={expanded}
  aria-label="{session.name}, {sessionStatus.label}, {session.paneCount} panes"
  class={`group relative overflow-hidden rounded-2xl border bg-surface-raised focus:outline-none focus-visible:ring-2 focus-visible:ring-border-focus focus-visible:ring-offset-2 focus-visible:ring-offset-surface-base ${
    pinned ? 'border-accent ring-1 ring-accent/30' : 'border-border'
  } ${dense ? 'p-3' : 'p-4'}`}
  on:keydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      dispatch('toggle', { sessionId: session.sessionId });
    }
  }}
>
  <div class="pointer-events-none absolute inset-0 bg-gradient-to-br from-accent/10 via-transparent to-status-success/10 opacity-0 transition group-hover:opacity-100"></div>
  <!-- Header row with toggle area and actions -->
  <div class="relative flex w-full flex-wrap items-center justify-between gap-4">
    <!-- Clickable toggle area -->
    <button
      type="button"
      class="flex flex-1 items-center gap-3 text-left"
      on:click={() => dispatch('toggle', { sessionId: session.sessionId })}
    >
      <div
        class={`flex h-10 w-10 items-center justify-center rounded-xl border border-border bg-surface-base ${
          dense ? 'h-9 w-9' : ''
        }`}
      >
        <span class={`status-dot ${sessionStatus.dot}`}></span>
      </div>
      <div>
        <p class={`font-semibold text-text-primary ${dense ? 'text-base' : 'text-lg'}`}>
          {session.name}
        </p>
        <p class="text-[11px] uppercase tracking-[0.25em] text-text-muted">
          Session · {session.sessionId.slice(0, 8)}
        </p>
      </div>
    </button>
    <!-- Actions and status (not inside toggle button) -->
    <div class="flex items-center gap-2 text-xs text-text-secondary">
      <!-- Pin button -->
      <button
        type="button"
        class={`rounded p-1 transition hover:bg-surface-base ${
          pinned ? 'text-accent' : 'text-text-subtle opacity-0 group-hover:opacity-100'
        }`}
        title={pinned ? 'Unpin session' : 'Pin session'}
        on:click={handlePin}
      >
        📌
      </button>
      <span class={`badge ${sessionStatus.badge}`}>
        {sessionStatus.label}
      </span>
      <span>{session.paneCount} panes</span>
      {#if lastSeen}
        <span class="hidden sm:inline">Seen {lastSeen}</span>
      {/if}
      <!-- Expand/collapse toggle -->
      <button
        type="button"
        class="text-lg text-text-subtle hover:text-text-secondary p-1"
        on:click={() => dispatch('toggle', { sessionId: session.sessionId })}
      >
        {expanded ? '▾' : '▸'}
      </button>
    </div>
  </div>

  <div class={`relative mt-3 grid gap-2 text-xs text-text-secondary ${dense ? 'sm:grid-cols-2' : 'sm:grid-cols-3'}`}>
    <div class="rounded-lg border border-border bg-surface-base px-3 py-2">
      <p class="label-sm">Active</p>
      <p class="mt-1 text-sm font-semibold text-status-success">{activeCount}</p>
    </div>
    <div class="rounded-lg border border-border bg-surface-base px-3 py-2">
      <p class="label-sm">Waiting</p>
      <p class="mt-1 text-sm font-semibold text-status-warning">{waitingCount}</p>
    </div>
    <div class="rounded-lg border border-border bg-surface-base px-3 py-2">
      <p class="label-sm">Idle</p>
      <p class="mt-1 text-sm font-semibold text-text-secondary">{idleCount}</p>
    </div>
  </div>

  {#if expanded}
    <div class="relative mt-4">
      <slot />
    </div>
  {/if}
</div>
