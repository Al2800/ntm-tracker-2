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

  const filters: { value: StatusFilter; label: string; count: () => number }[] = [
    { value: 'all', label: 'All', count: () => $sessions.length },
    { value: 'active', label: 'Active', count: () => $sessions.filter(s => s.status === 'active').length },
    { value: 'idle', label: 'Idle', count: () => $sessions.filter(s => s.status === 'idle').length },
    { value: 'ended', label: 'Ended', count: () => $sessions.filter(s => s.status === 'ended').length }
  ];

  // Search matching
  const matchesSearch = (session: Session, query: string) => {
    if (!query) return true;
    const haystack = `${session.name} ${session.sessionUid}`.toLowerCase();
    return haystack.includes(query.toLowerCase());
  };

  // Filter and sort sessions
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

  function handleSelect(session: Session) {
    selectSession(session.sessionUid);
    dispatch('focus', { session });
  }

  function handleAction(session: Session, action: string) {
    dispatch('action', { session, action });
  }
</script>

<div class="flex h-full flex-col">
  <!-- Filter chips -->
  <div class="mb-3 flex flex-wrap gap-1.5">
    {#each filters as filter}
      <button
        type="button"
        class="inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-2xs font-medium transition-colors"
        class:bg-accent-muted={statusFilter === filter.value}
        class:text-accent={statusFilter === filter.value}
        class:bg-surface-base={statusFilter !== filter.value}
        class:text-text-muted={statusFilter !== filter.value}
        class:hover:text-text-secondary={statusFilter !== filter.value}
        on:click={() => statusFilter = filter.value}
      >
        {filter.label}
        <span class="rounded-full bg-surface-base px-1.5 text-text-subtle">
          {filter.count()}
        </span>
      </button>
    {/each}
  </div>

  <!-- Sort dropdown -->
  <div class="mb-3 flex items-center justify-between">
    <span class="text-2xs text-text-subtle">
      {filteredSessions.length} session{filteredSessions.length !== 1 ? 's' : ''}
    </span>
    <select
      bind:value={sortBy}
      class="rounded border border-border bg-surface-base px-2 py-1 text-2xs text-text-secondary focus:border-border-focus focus:outline-none"
    >
      <option value="status">Sort: Status</option>
      <option value="name">Sort: Name</option>
      <option value="activity">Sort: Recent</option>
    </select>
  </div>

  <!-- Session list -->
  <div class="flex-1 space-y-1.5 overflow-y-auto">
    {#if filteredSessions.length === 0}
      <div class="rounded-lg border border-dashed border-border bg-surface-base p-4 text-center text-xs text-text-subtle">
        {#if searchQuery || statusFilter !== 'all'}
          No sessions match your filters
        {:else}
          No sessions yet
        {/if}
      </div>
    {:else}
      {#each filteredSessions as session (session.sessionUid)}
        <SessionItem
          {session}
          selected={session.sessionUid === $selectedSessionId}
          on:select={() => handleSelect(session)}
          on:action={(e) => handleAction(session, e.detail)}
        />
      {/each}
    {/if}
  </div>
</div>
