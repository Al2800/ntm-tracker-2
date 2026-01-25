<script lang="ts">
  import type { Session } from '../types';
  import { sessions, selectedSessionId, selectSession } from '../stores/sessions';
  import SessionCard from './SessionCard.svelte';
  import PaneList from './PaneList.svelte';

  export let query = '';
  export let dense = false;

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

  $: normalizedQuery = query.trim().toLowerCase();
  $: tokens = normalizedQuery.split(/\s+/).filter(Boolean);
  $: filteredSessions =
    tokens.length === 0
      ? $sessions
      : $sessions.filter((session) => tokens.every((token) => matchesToken(session, token)));
</script>

<div class={dense ? 'space-y-3' : 'space-y-4'}>
  {#if filteredSessions.length === 0}
    <div
      class={`rounded-xl border border-dashed border-slate-800 ${
        dense ? 'bg-slate-950/60 p-4 text-xs' : 'bg-slate-900/60 p-6 text-sm'
      } text-slate-400`}
    >
      No sessions match your search yet.
    </div>
  {:else}
    {#each filteredSessions as session (session.sessionUid)}
      <SessionCard
        {session}
        expanded={session.sessionUid === $selectedSessionId}
        dense={dense}
        on:toggle={(event) => handleToggle(event.detail.sessionUid)}
      >
        <PaneList panes={session.panes ?? []} dense={dense} />
      </SessionCard>
    {/each}
  {/if}
</div>
