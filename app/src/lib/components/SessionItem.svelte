<!--
  SessionItem.svelte
  Compact session card for sidebar list.
  Shows status, name, pane counts, and quick actions on hover.
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Session } from '$lib/types';

  export let session: Session;
  export let selected = false;
  export let pinned = false;
  export let muted = false;
  export let alertCount = 0;

  const dispatch = createEventDispatcher<{
    select: void;
    action: string;
  }>();

  // Status styling
  const statusDot: Record<string, string> = {
    active: 'bg-status-success',
    idle: 'bg-status-neutral',
    waiting: 'bg-status-warning',
    ended: 'bg-status-error',
    unknown: 'bg-text-subtle'
  };

  const formatAge = (timestamp?: number) => {
    if (!timestamp) return null;
    const now = Date.now();
    const ts = timestamp < 1_000_000_000_000 ? timestamp * 1000 : timestamp;
    const diffMs = Math.max(0, now - ts);
    const minutes = Math.floor(diffMs / 60000);
    if (minutes < 1) return 'now';
    if (minutes < 60) return `${minutes}m`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h`;
    const days = Math.floor(hours / 24);
    return `${days}d`;
  };

  $: panes = session.panes ?? [];
  $: activeCount = panes.filter(p => p.status === 'active').length;
  $: totalCount = panes.length;
  $: lastSeen = formatAge(session.lastSeenAt);

  let showActions = false;

  function handleAction(action: string) {
    dispatch('action', action);
  }
</script>

<div
  data-session-item
  id={`session-item-${session.sessionId}`}
  role="option"
  tabindex="0"
  class="session-item group relative w-full rounded-lg border p-2.5 text-left transition-all focus-ring cursor-pointer"
  class:border-accent={selected}
  class:bg-accent-muted={selected}
  class:border-border={!selected}
  class:bg-surface-base={!selected}
  class:hover:border-border-strong={!selected}
  class:hover:bg-surface-raised={!selected}
  on:click={() => dispatch('select')}
  on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); dispatch('select'); } }}
  on:mouseenter={() => showActions = true}
  on:mouseleave={() => showActions = false}
  on:focus={() => showActions = true}
  on:blur={(e) => {
    // Don't hide if focus moved to a child element (e.g., copy button)
    if (e.currentTarget.contains(e.relatedTarget as Node)) return;
    showActions = false;
  }}
  aria-label="Session {session.name}, status {session.status}, {totalCount} pane{totalCount !== 1 ? 's' : ''}"
  aria-selected={selected}
>
  <div class="flex items-start gap-2.5">
    <!-- Status indicator -->
    <div class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-md border border-border bg-surface-raised">
      <span class="h-1.5 w-1.5 rounded-full {statusDot[session.status]}"></span>
    </div>

    <!-- Session info -->
    <div class="min-w-0 flex-1">
      <div class="flex items-center justify-between gap-2">
        <div class="flex min-w-0 items-center gap-1">
          <p class="truncate text-sm font-medium text-text-primary">
            {session.name}
          </p>
          {#if pinned}
            <span class="text-2xs text-accent" title="Pinned" aria-label="Pinned session">
              📌
            </span>
          {/if}
        </div>
        <div class="flex shrink-0 items-center gap-2">
          {#if alertCount > 0}
            <span
              class={`badge badge-warning text-[10px] ${muted ? 'opacity-60' : ''}`}
              aria-label={`${alertCount} pending escalation${alertCount === 1 ? '' : 's'}`}
            >
              {alertCount} alert{alertCount === 1 ? '' : 's'}
            </span>
          {/if}
          {#if lastSeen}
            <span class="text-2xs text-text-subtle">
              {lastSeen}
            </span>
          {/if}
        </div>
      </div>
      <div class="mt-0.5 flex items-center gap-2 text-2xs text-text-muted">
        <span class="font-mono">{session.sessionId.slice(0, 8)}</span>
        <span class="text-text-subtle">·</span>
        <span>
          {#if activeCount > 0}
            <span class="text-status-success-text">{activeCount}</span>/{totalCount}
          {:else}
            {totalCount}
          {/if}
          pane{totalCount !== 1 ? 's' : ''}
        </span>
      </div>
    </div>
  </div>

  <!-- Quick actions (visible on hover) -->
  {#if showActions}
    <div class="absolute right-1.5 top-1.5 flex gap-1 animate-fade-in">
      <button
        type="button"
        class="rounded bg-surface-elevated p-1 text-text-muted hover:bg-surface-overlay hover:text-text-primary focus-ring"
        title="Attach to session"
        aria-label="Copy attach command for {session.name}"
        on:click|stopPropagation={() => handleAction('attach')}
      >
        <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
        </svg>
      </button>
      <button
        type="button"
        class={`rounded bg-surface-elevated p-1 focus-ring ${
          pinned ? 'text-accent' : 'text-text-muted hover:bg-surface-overlay hover:text-text-primary'
        }`}
        title={pinned ? 'Unpin session' : 'Pin session'}
        aria-label={pinned ? 'Unpin session' : 'Pin session'}
        on:click|stopPropagation={() => handleAction('pin')}
      >
        📌
      </button>
      <button
        type="button"
        class={`rounded bg-surface-elevated p-1 focus-ring ${
          muted ? 'text-status-warning-text' : 'text-text-muted hover:bg-surface-overlay hover:text-text-primary'
        }`}
        title={muted ? 'Unmute alerts' : 'Mute alerts'}
        aria-label={muted ? 'Unmute alerts for session' : 'Mute alerts for session'}
        on:click|stopPropagation={() => handleAction('mute')}
      >
        {muted ? '🔕' : '🔔'}
      </button>
      <button
        type="button"
        class="rounded bg-surface-elevated p-1 text-text-muted hover:bg-status-error-muted hover:text-status-error-text focus-ring"
        title="Kill session"
        aria-label="Kill session {session.name}"
        on:click|stopPropagation={() => handleAction('kill')}
      >
        ✕
      </button>
    </div>
  {/if}
</div>

<style>
  /* CSS containment for scroll performance with large lists */
  .session-item {
    contain: layout style paint;
  }
</style>
