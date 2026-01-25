<script lang="ts">
  import type { Session } from '../types';
  import { sessions, selectedSessionId, selectSession, pinnedSessionIds, togglePinSession } from '../stores/sessions';
  import SessionCard from './SessionCard.svelte';
  import PaneList from './PaneList.svelte';

  export let query = '';
  export let dense = false;
  export let showControls = true;

  type StatusFilter = 'all' | 'active' | 'idle' | 'waiting' | 'ended';
  type SortOption = 'status' | 'name' | 'activity' | 'panes';

  let statusFilter: StatusFilter = 'all';
  let sortBy: SortOption = 'status';
  let showPinnedOnly = false;

  const handleToggle = (sessionUid: string) => {
    selectSession($selectedSessionId === sessionUid ? null : sessionUid);
  };

  const isSubsequence = (needle: string, haystack: string) => {
    let needleIndex = 0;
    for (let haystackIndex = 0; haystackIndex < haystack.length; haystackIndex += 1) {
      if (haystack[haystackIndex] === needle[needleIndex]) {
        needleIndex += 1;
        if (needleIndex >= needle.length) {
          return true;
        }
      }
    }
    return needle.length === 0;
  };

  const matchesToken = (session: Session, token: string) => {
    const haystack = `${session.name} ${session.sessionUid}`.toLowerCase();
    return haystack.includes(token) || isSubsequence(token, haystack);
  };

  const hasWaitingPanes = (session: Session): boolean => {
    return (session.panes ?? []).some((p) => p.status === 'waiting');
  };

  const statusRank: Record<string, number> = {
    active: 0,
    idle: 1,
    waiting: 2,
    ended: 3,
    unknown: 4
  };

  const sortSessions = (list: Session[], sortOption: SortOption): Session[] => {
    return [...list].sort((a, b) => {
      // Pinned sessions always come first
      const aPinned = $pinnedSessionIds.has(a.sessionUid);
      const bPinned = $pinnedSessionIds.has(b.sessionUid);
      if (aPinned !== bPinned) return aPinned ? -1 : 1;

      switch (sortOption) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'activity':
          return (b.lastSeenAt ?? 0) - (a.lastSeenAt ?? 0);
        case 'panes':
          return b.paneCount - a.paneCount;
        case 'status':
        default:
          const rankA = statusRank[a.status] ?? 4;
          const rankB = statusRank[b.status] ?? 4;
          if (rankA !== rankB) return rankA - rankB;
          return a.name.localeCompare(b.name);
      }
    });
  };

  $: normalizedQuery = query.trim().toLowerCase();
  $: tokens = normalizedQuery.split(/\s+/).filter(Boolean);

  // Filter by search query
  $: queryFiltered =
    tokens.length === 0
      ? $sessions
      : $sessions.filter((session) => tokens.every((token) => matchesToken(session, token)));

  // Filter by status
  $: statusFiltered = queryFiltered.filter((session) => {
    if (statusFilter === 'all') return true;
    if (statusFilter === 'waiting') return hasWaitingPanes(session);
    return session.status === statusFilter;
  });

  // Filter by pinned
  $: pinnedFiltered = showPinnedOnly
    ? statusFiltered.filter((s) => $pinnedSessionIds.has(s.sessionUid))
    : statusFiltered;

  // Apply sorting
  $: filteredSessions = sortSessions(pinnedFiltered, sortBy);

  // Counts for filter badges
  $: counts = {
    all: queryFiltered.length,
    active: queryFiltered.filter((s) => s.status === 'active').length,
    idle: queryFiltered.filter((s) => s.status === 'idle').length,
    waiting: queryFiltered.filter((s) => hasWaitingPanes(s)).length,
    ended: queryFiltered.filter((s) => s.status === 'ended').length
  };

  const filterOptions: { value: StatusFilter; label: string; color: string }[] = [
    { value: 'all', label: 'All', color: 'bg-slate-500/20 text-slate-300 ring-slate-500/40' },
    { value: 'active', label: 'Active', color: 'bg-emerald-500/20 text-emerald-300 ring-emerald-500/40' },
    { value: 'idle', label: 'Idle', color: 'bg-slate-500/20 text-slate-300 ring-slate-500/40' },
    { value: 'waiting', label: 'Waiting', color: 'bg-amber-500/20 text-amber-300 ring-amber-500/40' },
    { value: 'ended', label: 'Ended', color: 'bg-rose-500/20 text-rose-300 ring-rose-500/40' }
  ];

  const sortOptions: { value: SortOption; label: string }[] = [
    { value: 'status', label: 'Status' },
    { value: 'name', label: 'Name' },
    { value: 'activity', label: 'Recent Activity' },
    { value: 'panes', label: 'Pane Count' }
  ];
</script>

<div class={dense ? 'space-y-3' : 'space-y-4'}>
  <!-- Filter & Sort Controls -->
  {#if showControls && !dense}
    <div class="flex flex-wrap items-center gap-3">
      <!-- Status filter chips -->
      <div class="flex flex-wrap gap-1.5">
        {#each filterOptions as opt (opt.value)}
          <button
            type="button"
            class={`rounded-full px-2.5 py-1 text-[11px] font-medium ring-1 transition ${
              statusFilter === opt.value
                ? opt.color
                : 'bg-slate-900/60 text-slate-400 ring-slate-700/50 hover:ring-slate-600'
            }`}
            on:click={() => (statusFilter = opt.value)}
          >
            {opt.label}
            {#if counts[opt.value] > 0}
              <span class="ml-1 opacity-70">({counts[opt.value]})</span>
            {/if}
          </button>
        {/each}
      </div>

      <div class="flex-1"></div>

      <!-- Pinned toggle -->
      <button
        type="button"
        class={`rounded-full px-2.5 py-1 text-[11px] font-medium ring-1 transition ${
          showPinnedOnly
            ? 'bg-sky-500/20 text-sky-300 ring-sky-500/40'
            : 'bg-slate-900/60 text-slate-400 ring-slate-700/50 hover:ring-slate-600'
        }`}
        on:click={() => (showPinnedOnly = !showPinnedOnly)}
      >
        📌 Pinned
        {#if $pinnedSessionIds.size > 0}
          <span class="ml-1 opacity-70">({$pinnedSessionIds.size})</span>
        {/if}
      </button>

      <!-- Sort dropdown -->
      <div class="flex items-center gap-2">
        <span class="text-[10px] uppercase tracking-wider text-slate-500">Sort</span>
        <select
          bind:value={sortBy}
          class="rounded-lg border border-slate-700/60 bg-slate-900/80 px-2 py-1 text-xs text-slate-200 focus:border-sky-500/50 focus:outline-none"
        >
          {#each sortOptions as opt (opt.value)}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>
    </div>
  {/if}

  <!-- Sessions list -->
  {#if filteredSessions.length === 0}
    <div
      class={`rounded-xl border border-dashed border-slate-800 ${
        dense ? 'bg-slate-950/60 p-4 text-xs' : 'bg-slate-900/60 p-6 text-sm'
      } text-slate-400`}
    >
      {#if showPinnedOnly}
        No pinned sessions. Click 📌 on a session to pin it.
      {:else if statusFilter !== 'all'}
        No {statusFilter} sessions match your search.
      {:else}
        No sessions match your search yet.
      {/if}
    </div>
  {:else}
    {#each filteredSessions as session (session.sessionUid)}
      <SessionCard
        {session}
        expanded={session.sessionUid === $selectedSessionId}
        dense={dense}
        pinned={$pinnedSessionIds.has(session.sessionUid)}
        on:toggle={(event) => handleToggle(event.detail.sessionUid)}
        on:pin={(event) => togglePinSession(event.detail.sessionUid)}
      >
        <PaneList panes={session.panes ?? []} dense={dense} />
      </SessionCard>
    {/each}
  {/if}
</div>
