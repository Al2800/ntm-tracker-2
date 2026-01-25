<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Session } from '../types';

  export let session: Session;
  export let expanded = false;
  export let dense = false;
  export let pinned = false;

  const dispatch = createEventDispatcher<{
    toggle: { sessionUid: string };
    pin: { sessionUid: string };
  }>();

  const handlePin = (e: Event) => {
    e.stopPropagation();
    dispatch('pin', { sessionUid: session.sessionUid });
  };

  const statusClasses: Record<Session['status'], string> = {
    active: 'bg-emerald-500/15 text-emerald-200 ring-1 ring-emerald-400/40',
    idle: 'bg-slate-500/15 text-slate-200 ring-1 ring-slate-500/40',
    ended: 'bg-rose-500/15 text-rose-200 ring-1 ring-rose-400/40',
    unknown: 'bg-slate-700/40 text-slate-300 ring-1 ring-slate-600/40'
  };

  const statusDot: Record<Session['status'], string> = {
    active: 'bg-emerald-400',
    idle: 'bg-slate-300',
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

  $: panes = session.panes ?? [];
  $: activeCount = panes.filter((pane) => pane.status === 'active').length;
  $: idleCount = panes.filter((pane) => pane.status === 'idle').length;
  $: waitingCount = panes.filter((pane) => pane.status === 'waiting').length;
  $: lastSeen = formatAge(session.lastSeenAt);
</script>

<div
  data-session-card
  id="session-{session.sessionUid}"
  tabindex="0"
  role="option"
  aria-selected={expanded}
  aria-label="{session.name}, {session.status}, {session.paneCount} panes"
  class={`group relative overflow-hidden rounded-2xl border bg-slate-900/60 focus:outline-none focus-visible:ring-2 focus-visible:ring-border-focus focus-visible:ring-offset-2 focus-visible:ring-offset-surface-base ${
    pinned ? 'border-sky-500/40 ring-1 ring-sky-500/20' : 'border-slate-800/80'
  } ${dense ? 'p-3' : 'p-4'}`}
  on:keydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      dispatch('toggle', { sessionUid: session.sessionUid });
    }
  }}
>
  <div class="pointer-events-none absolute inset-0 bg-gradient-to-br from-sky-500/10 via-transparent to-emerald-500/10 opacity-0 transition group-hover:opacity-100"></div>
  <!-- Header row with toggle area and actions -->
  <div class="relative flex w-full flex-wrap items-center justify-between gap-4">
    <!-- Clickable toggle area -->
    <button
      type="button"
      class="flex flex-1 items-center gap-3 text-left"
      on:click={() => dispatch('toggle', { sessionUid: session.sessionUid })}
    >
      <div
        class={`flex h-10 w-10 items-center justify-center rounded-xl border border-slate-700/70 bg-slate-950/60 ${
          dense ? 'h-9 w-9' : ''
        }`}
      >
        <span class={`h-2 w-2 rounded-full ${statusDot[session.status]}`}></span>
      </div>
      <div>
        <p class={`font-semibold text-white ${dense ? 'text-base' : 'text-lg'}`}>
          {session.name}
        </p>
        <p class="text-[11px] uppercase tracking-[0.25em] text-slate-400">
          Session · {session.sessionUid.slice(0, 8)}
        </p>
      </div>
    </button>
    <!-- Actions and status (not inside toggle button) -->
    <div class="flex items-center gap-2 text-xs text-slate-400">
      <!-- Pin button -->
      <button
        type="button"
        class={`rounded p-1 transition hover:bg-slate-800/60 ${
          pinned ? 'text-sky-400' : 'text-slate-500 opacity-0 group-hover:opacity-100'
        }`}
        title={pinned ? 'Unpin session' : 'Pin session'}
        on:click={handlePin}
      >
        📌
      </button>
      <span class={`rounded-full px-2.5 py-1 ${statusClasses[session.status]}`}>
        {session.status}
      </span>
      <span>{session.paneCount} panes</span>
      {#if lastSeen}
        <span class="hidden sm:inline">Seen {lastSeen}</span>
      {/if}
      <!-- Expand/collapse toggle -->
      <button
        type="button"
        class="text-lg text-slate-500 hover:text-slate-300 p-1"
        on:click={() => dispatch('toggle', { sessionUid: session.sessionUid })}
      >
        {expanded ? '▾' : '▸'}
      </button>
    </div>
  </div>

  <div class={`relative mt-3 grid gap-2 text-xs text-slate-300 ${dense ? 'sm:grid-cols-2' : 'sm:grid-cols-3'}`}>
    <div class="rounded-lg border border-slate-800/80 bg-slate-950/50 px-3 py-2">
      <p class="text-[10px] uppercase tracking-[0.2em] text-slate-500">Active</p>
      <p class="mt-1 text-sm font-semibold text-emerald-200">{activeCount}</p>
    </div>
    <div class="rounded-lg border border-slate-800/80 bg-slate-950/50 px-3 py-2">
      <p class="text-[10px] uppercase tracking-[0.2em] text-slate-500">Waiting</p>
      <p class="mt-1 text-sm font-semibold text-amber-200">{waitingCount}</p>
    </div>
    <div class="rounded-lg border border-slate-800/80 bg-slate-950/50 px-3 py-2">
      <p class="text-[10px] uppercase tracking-[0.2em] text-slate-500">Idle</p>
      <p class="mt-1 text-sm font-semibold text-slate-200">{idleCount}</p>
    </div>
  </div>

  {#if expanded}
    <div class="relative mt-4">
      <slot />
    </div>
  {/if}
</div>
