<svelte:head>
  <title>NTM Tracker</title>
</svelte:head>

<script lang="ts">
  import { page } from '$app/stores';
  import { connectionState, lastConnectionError } from '$lib/stores/connection';
  import { sessions, selectedSession, selectSession } from '$lib/stores/sessions';
  import OutputPreview from '$lib/components/OutputPreview.svelte';
  import type { Session } from '$lib/types';
  import { onDestroy, onMount, tick } from 'svelte';

  let query = '';
  let searchInput: HTMLInputElement | null = null;
  let mounted = false;
  let selectedPaneId: string | null = null;
  let lastSelectedSessionId: string | null = null;

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

  $: focusRequested = $page.url.searchParams.get('focusSearch') === '1';
  $: if (mounted && focusRequested) {
    void tick().then(() => searchInput?.focus());
  }

  $: if (($selectedSession?.sessionUid ?? null) !== lastSelectedSessionId) {
    lastSelectedSessionId = $selectedSession?.sessionUid ?? null;
    selectedPaneId = null;
  }

  onMount(() => {
    mounted = true;
    const onKeydown = (event: KeyboardEvent) => {
      if (!(event.key === 'k' || event.key === 'K')) {
        return;
      }
      if (!(event.ctrlKey || event.metaKey)) {
        return;
      }

      event.preventDefault();
      searchInput?.focus();
    };

    window.addEventListener('keydown', onKeydown);
    return () => {
      window.removeEventListener('keydown', onKeydown);
    };
  });

  onDestroy(() => {
    mounted = false;
  });
</script>

<main class="min-h-screen bg-slate-950 text-slate-100">
  <div class="mx-auto max-w-5xl px-6 py-16">
    {#if $connectionState !== 'connected'}
      <div class="mb-6 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm">
        <div class="flex items-center justify-between gap-3">
          <span class="font-semibold text-amber-200">
            Connection: {$connectionState}
          </span>
          {#if $lastConnectionError}
            <span class="text-amber-100/80">{$lastConnectionError}</span>
          {/if}
        </div>
      </div>
    {/if}
    <p class="text-sm uppercase tracking-[0.3em] text-slate-400">NTM Tracker</p>
    <h1 class="mt-4 text-4xl font-semibold text-white">
      System tray telemetry for your NTM sessions
    </h1>
    <p class="mt-4 max-w-2xl text-lg text-slate-300">
      This UI will surface live sessions, compact events, escalations, and usage
      analytics once the daemon is wired up.
    </p>
    <div class="mt-10 grid gap-4 sm:grid-cols-3">
      <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
        <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Sessions</p>
        <p class="mt-3 text-2xl font-semibold text-white">{$sessions.length}</p>
      </div>
      <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
        <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Compacts</p>
        <p class="mt-3 text-2xl font-semibold text-white">0 today</p>
      </div>
      <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
        <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Escalations</p>
        <p class="mt-3 text-2xl font-semibold text-white">None</p>
      </div>
    </div>

    <section class="mt-12 rounded-xl border border-slate-800 bg-slate-900/60 p-6">
      <div class="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 class="text-lg font-semibold text-white">Sessions</h2>
          <p class="mt-1 text-sm text-slate-300/80">
            Search by name or session UID. Click a session to focus it.
          </p>
        </div>
        <label class="grid gap-2 text-sm text-slate-200">
          Search
          <input
            class="w-64 rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
            placeholder="type to filter... (Ctrl+K)"
            bind:value={query}
            bind:this={searchInput}
          />
        </label>
      </div>

      <div class="mt-6 grid gap-3 sm:grid-cols-2">
        {#if filteredSessions.length === 0}
          <div class="text-sm text-slate-300/80">No sessions match your search.</div>
        {:else}
          {#each filteredSessions as session (session.sessionUid)}
            <button
              class="rounded-lg border border-slate-800 bg-slate-950/40 px-4 py-3 text-left hover:bg-slate-950/60"
              on:click={() => selectSession(session.sessionUid)}
            >
              <div class="flex items-center justify-between gap-3">
                <div>
                  <p class="font-semibold text-white">{session.name ?? session.sessionUid}</p>
                  <p class="mt-1 text-xs text-slate-400">{session.sessionUid}</p>
                </div>
                <span class="rounded-full bg-slate-800 px-3 py-1 text-xs text-slate-200">
                  {session.status}
                </span>
              </div>
            </button>
          {/each}
        {/if}
      </div>

	      {#if $selectedSession}
	        <div class="mt-6 rounded-lg border border-slate-800 bg-slate-950/40 p-4">
	          <div class="flex items-center justify-between">
	            <div>
	              <p class="text-sm uppercase tracking-[0.2em] text-slate-400">Selected</p>
              <p class="mt-2 text-xl font-semibold text-white">
                {$selectedSession.name ?? $selectedSession.sessionUid}
              </p>
              <p class="mt-1 text-xs text-slate-400">{$selectedSession.sessionUid}</p>
	            </div>
	            <button
	              class="rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm hover:bg-slate-800"
	              on:click={() => {
	                selectSession(null);
	                selectedPaneId = null;
	              }}
	            >
	              Clear
	            </button>
	          </div>

	          <div class="mt-4">
	            <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Panes</p>
	            {#if ($selectedSession.panes ?? []).length === 0}
	              <p class="mt-2 text-sm text-slate-300/80">
	                No pane details available yet for this session.
	              </p>
	            {:else}
	              <div class="mt-3 grid gap-2 sm:grid-cols-2">
	                {#each ($selectedSession.panes ?? []) as pane (pane.paneUid)}
	                  <button
	                    class={`rounded-lg border px-3 py-2 text-left text-sm hover:bg-slate-950/60 ${
	                      selectedPaneId === pane.paneUid
	                        ? 'border-sky-500/60 bg-sky-500/10'
	                        : 'border-slate-800 bg-slate-950/40'
	                    }`}
	                    on:click={() => (selectedPaneId = pane.paneUid)}
	                  >
	                    <div class="flex items-center justify-between gap-2">
	                      <span class="font-semibold text-white">Pane {pane.index}</span>
	                      <span class="rounded-full bg-slate-800 px-2 py-0.5 text-xs text-slate-200">
	                        {pane.status}
	                      </span>
	                    </div>
	                    <div class="mt-1 text-xs text-slate-400">
	                      {#if pane.agentType}
	                        <span class="uppercase tracking-[0.2em]">{pane.agentType}</span>
	                        <span class="mx-2 text-slate-600">·</span>
	                      {/if}
	                      <span class="font-mono">{pane.paneUid}</span>
	                    </div>
	                    {#if pane.currentCommand}
	                      <div class="mt-1 text-xs text-slate-300/80">cmd: {pane.currentCommand}</div>
	                    {/if}
	                  </button>
	                {/each}
	              </div>

	              {#if selectedPaneId}
	                <div class="mt-4">
	                  <OutputPreview paneId={selectedPaneId} />
	                </div>
	              {/if}
	            {/if}
	          </div>
	        </div>
	      {/if}
	    </section>
	  </div>
</main>
