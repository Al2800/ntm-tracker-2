<script lang="ts">
  import { sessions, selectedSessionId, selectSession } from '../stores/sessions';
  import SessionCard from './SessionCard.svelte';
  import PaneList from './PaneList.svelte';

  const handleToggle = (sessionUid: string) => {
    selectSession($selectedSessionId === sessionUid ? null : sessionUid);
  };
</script>

<div class="space-y-4">
  {#if $sessions.length === 0}
    <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-6 text-slate-400">
      No sessions reported yet.
    </div>
  {:else}
    {#each $sessions as session (session.sessionUid)}
      <SessionCard
        {session}
        expanded={session.sessionUid === $selectedSessionId}
        on:toggle={(event) => handleToggle(event.detail.sessionUid)}
      >
        <PaneList panes={session.panes ?? []} />
      </SessionCard>
    {/each}
  {/if}
</div>
