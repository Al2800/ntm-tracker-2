<!--
  SessionsHub.svelte
  Sidebar sessions list with filters, sorting, and quick actions.
  Designed for the sidebar slot in DashboardLayout.
  See docs/information-architecture.md for design rationale.
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Session } from '$lib/types';
  import { sessions, selectedSessionId, selectSession } from '$lib/stores/sessions';
  import SessionItem from './SessionItem.svelte';
  import EmptyState from './states/EmptyState.svelte';

  export let searchQuery = '';

  const dispatch = createEventDispatcher<{
    focus: { session: Session };
    action: { session: Session; action: string };
  }>();

  // Filter state
  type StatusFilter = 'all' | 'active' | 'idle' | 'ended';
  let statusFilter: StatusFilter = 'all';

  // Sort state
  type SortOption = 'status' | 'name' | 'activity';
  let sortBy: SortOption = 'status';

  const statusRank: Record<string, number> = {
    active: 0,
    idle: 1,
    waiting: 2,
    ended: 3,
    unknown: 4
  };

  // Memoize filter counts to avoid recalculating on every render
  $: statusCounts = {
    all: $sessions.length,
    active: $sessions.filter(s => s.status === 'active').length,
    idle: $sessions.filter(s => s.status === 'idle').length,
    ended: $sessions.filter(s => s.status === 'ended').length
  };

  const filterLabels: { value: StatusFilter; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'active', label: 'Active' },
    { value: 'idle', label: 'Idle' },
    { value: 'ended', label: 'Ended' }
  ];

  // Render limit for large lists (show "load more" after this)
  const INITIAL_RENDER_LIMIT = 50;
  let renderLimit = INITIAL_RENDER_LIMIT;

  // Search matching
  const matchesSearch = (session: Session, query: string) => {
    if (!query) return true;
    const haystack = `${session.name} ${session.sessionId}`.toLowerCase();
    return haystack.includes(query.toLowerCase());
  };

  // Filter and sort sessions (memoized)
  $: filteredSessions = $sessions
    .filter(session => {
      // Status filter
      if (statusFilter !== 'all' && session.status !== statusFilter) return false;
      // Search filter
      if (!matchesSearch(session, searchQuery)) return false;
      return true;
    })
    .sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'activity':
          return (b.lastSeenAt ?? 0) - (a.lastSeenAt ?? 0);
        case 'status':
        default:
          const rankDiff = (statusRank[a.status] ?? 4) - (statusRank[b.status] ?? 4);
          if (rankDiff !== 0) return rankDiff;
          return a.name.localeCompare(b.name);
      }
    });

  // Visible sessions (limited for performance with large lists)
  $: visibleSessions = filteredSessions.slice(0, renderLimit);
  $: hasMore = filteredSessions.length > renderLimit;

  // Reset render limit when filters change
  $: if (statusFilter || searchQuery || sortBy) {
    renderLimit = INITIAL_RENDER_LIMIT;
  }

  function showMore() {
    renderLimit += INITIAL_RENDER_LIMIT;
  }

  function handleSelect(session: Session) {
    selectSession(session.sessionId);
    dispatch('focus', { session });
  }

  function handleAction(session: Session, action: string) {
    dispatch('action', { session, action });
  }
</script>

<div class="flex h-full flex-col">
  <!-- Filter chips -->
  <div class="mb-3 flex flex-wrap gap-1.5" role="group" aria-label="Filter sessions by status">
    {#each filterLabels as filter}
      <button
        type="button"
        class="inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-2xs font-medium transition-colors focus-ring"
        class:bg-accent-muted={statusFilter === filter.value}
        class:text-accent={statusFilter === filter.value}
        class:bg-surface-base={statusFilter !== filter.value}
        class:text-text-muted={statusFilter !== filter.value}
        class:hover:text-text-secondary={statusFilter !== filter.value}
        on:click={() => statusFilter = filter.value}
        aria-pressed={statusFilter === filter.value}
        aria-label="Filter by {filter.label} sessions, {statusCounts[filter.value]} available"
      >
        {filter.label}
        <span class="rounded-full bg-surface-base px-1.5 text-text-subtle" aria-hidden="true">
          {statusCounts[filter.value]}
        </span>
      </button>
    {/each}
  </div>

  <!-- Sort dropdown -->
  <div class="mb-3 flex items-center justify-between">
    <span class="text-2xs text-text-subtle" aria-live="polite">
      {filteredSessions.length} session{filteredSessions.length !== 1 ? 's' : ''}
    </span>
    <label class="sr-only" for="session-sort">Sort sessions by</label>
    <select
      id="session-sort"
      bind:value={sortBy}
      class="rounded border border-border bg-surface-base px-2 py-1 text-2xs text-text-secondary focus:border-border-focus focus:outline-none focus:ring-1 focus:ring-border-focus"
      aria-label="Sort sessions by"
    >
      <option value="status">Sort: Status</option>
      <option value="name">Sort: Name</option>
      <option value="activity">Sort: Recent</option>
    </select>
  </div>

  <!-- Session list (with scroll optimization) -->
  <div
    class="flex-1 space-y-1.5 overflow-y-auto"
    role="list"
    aria-label="Session list"
  >
    {#if filteredSessions.length === 0}
      <EmptyState
        icon={searchQuery || statusFilter !== 'all' ? 'search' : 'sessions'}
        title={searchQuery || statusFilter !== 'all' ? 'No sessions match your filters' : 'No sessions yet'}
        description={searchQuery || statusFilter !== 'all' ? 'Try adjusting your search or filters.' : 'Sessions will appear when NTM detects running tmux sessions.'}
        compact
      />
    {:else}
      {#each visibleSessions as session (session.sessionId)}
        <SessionItem
          {session}
          selected={session.sessionId === $selectedSessionId}
          on:select={() => handleSelect(session)}
          on:action={(e) => handleAction(session, e.detail)}
        />
      {/each}
      {#if hasMore}
        <div class="pt-2 pb-1 text-center">
          <button
            type="button"
            class="btn btn-sm btn-ghost text-2xs"
            on:click={showMore}
          >
            Show more ({filteredSessions.length - renderLimit} remaining)
          </button>
        </div>
      {/if}
    {/if}
  </div>
</div>
