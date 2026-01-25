<svelte:head>
  <title>NTM Tracker</title>
</svelte:head>

<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onDestroy, onMount, tick } from 'svelte';
  import { connectionState, lastConnectionError } from '$lib/stores/connection';
  import { events } from '$lib/stores/events';
  import { sessions, selectedSession, selectSession } from '$lib/stores/sessions';
  import { settings, updateSettings } from '$lib/stores/settings';
  import OverviewCards from '$lib/components/OverviewCards.svelte';
  import SessionList from '$lib/components/SessionList.svelte';
  import PaneList from '$lib/components/PaneList.svelte';
  import ActivityGraph from '$lib/components/ActivityGraph.svelte';
  import EscalationPanel from '$lib/components/EscalationPanel.svelte';
  import TimelinePanel from '$lib/components/TimelinePanel.svelte';
  import OutputPreview from '$lib/components/OutputPreview.svelte';

  let query = '';
  let searchInput: HTMLInputElement | null = null;
  let mounted = false;
  let selectedPaneId: string | null = null;
  let lastSelectedSessionId: string | null = null;

  const connectionLabel: Record<string, string> = {
    connected: 'Connected',
    connecting: 'Connecting',
    reconnecting: 'Reconnecting',
    degraded: 'Degraded',
    disconnected: 'Disconnected'
  };

  const connectionBadge: Record<string, string> = {
    connected: 'bg-emerald-500/15 text-emerald-200 ring-1 ring-emerald-400/40',
    connecting: 'bg-sky-500/15 text-sky-200 ring-1 ring-sky-400/40',
    reconnecting: 'bg-amber-500/15 text-amber-200 ring-1 ring-amber-400/40',
    degraded: 'bg-rose-500/15 text-rose-200 ring-1 ring-rose-400/40',
    disconnected: 'bg-slate-500/15 text-slate-200 ring-1 ring-slate-500/40'
  };

  const statusRank: Record<string, number> = {
    active: 0,
    idle: 1,
    ended: 2,
    unknown: 3
  };

  const formatQuietHours = (start: number, end: number) => `${start.toString().padStart(2, '0')}:00–${end.toString().padStart(2, '0')}:00`;

  $: normalizedQuery = query.trim().toLowerCase();
  $: focusRequested = $page.url.searchParams.get('focusSearch') === '1';
  $: compactMode =
    $page.url.searchParams.get('view') === 'compact' ||
    $page.url.searchParams.get('compact') === '1';

  $: if (mounted && focusRequested) {
    void tick().then(() => searchInput?.focus());
  }

  $: if (($selectedSession?.sessionUid ?? null) !== lastSelectedSessionId) {
    lastSelectedSessionId = $selectedSession?.sessionUid ?? null;
    selectedPaneId = null;
  }

  $: sortedSessions = [...$sessions].sort((a, b) => {
    const rank = (session: typeof a) => statusRank[session.status] ?? 4;
    if (rank(a) !== rank(b)) return rank(a) - rank(b);
    return a.name.localeCompare(b.name);
  });

  $: traySessions = sortedSessions.slice(0, 4);
  $: pendingEscalations = $events.filter(
    (event) => event.type === 'escalation' && (event.status ?? 'pending') === 'pending'
  );

  const toggleNotifications = () => {
    updateSettings({ showNotifications: !$settings.showNotifications });
  };

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
  <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.16),_rgba(15,23,42,0.2),_transparent_65%)]"></div>
  <div class="relative mx-auto max-w-6xl px-6 py-12">
    <header class="flex flex-wrap items-center justify-between gap-6">
      <div>
        <p class="text-xs uppercase tracking-[0.4em] text-slate-400">NTM Tracker</p>
        <h1 class="mt-4 text-4xl font-semibold text-white">Session intelligence, at a glance.</h1>
        <p class="mt-3 max-w-2xl text-base text-slate-300">
          Monitor NTM sessions, compact events, and escalations with a crisp, always-on view — designed to
          feel right at home in your system tray.
        </p>
      </div>
      <div class="flex flex-col items-start gap-2 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4">
        <span class="text-xs uppercase tracking-[0.2em] text-slate-400">Connection</span>
        <span class={`rounded-full px-3 py-1 text-xs ${connectionBadge[$connectionState]}`}>
          {connectionLabel[$connectionState]}
        </span>
        {#if $lastConnectionError}
          <span class="text-xs text-slate-500">{$lastConnectionError}</span>
        {/if}
      </div>
    </header>

    <div class="mt-10 flex flex-wrap items-center gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4">
      <div class="flex min-w-[240px] flex-1 items-center gap-3">
        <span class="text-xs uppercase tracking-[0.3em] text-slate-500">Search</span>
        <input
          class="w-full rounded-lg border border-slate-700/80 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500"
          placeholder="Find a session or UID… (Ctrl+K)"
          bind:value={query}
          bind:this={searchInput}
        />
      </div>
      <div class="flex flex-wrap items-center gap-2 text-xs text-slate-300">
        <button
          class="rounded-lg border border-slate-700/80 bg-slate-950 px-3 py-2 text-slate-100 transition hover:border-slate-500"
          type="button"
          on:click={() => goto('/settings')}
        >
          Settings
        </button>
        <button
          class={`rounded-lg border px-3 py-2 text-slate-100 transition ${
            $settings.showNotifications
              ? 'border-emerald-500/60 bg-emerald-500/10 text-emerald-100'
              : 'border-slate-700/80 bg-slate-950 text-slate-400'
          }`}
          type="button"
          on:click={toggleNotifications}
        >
          {$settings.showNotifications ? 'Notifications on' : 'Notifications muted'}
        </button>
        <button
          class="rounded-lg border border-slate-700/80 bg-slate-950 px-3 py-2 text-slate-100 transition hover:border-slate-500"
          type="button"
          on:click={() => goto('/wizard')}
        >
          Setup wizard
        </button>
      </div>
    </div>

    <section class="mt-10">
      <OverviewCards />
    </section>

    {#if compactMode}
      <section class="mt-10 grid gap-6">
        <div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5">
          <div class="flex items-center justify-between">
            <div>
              <h2 class="text-sm font-semibold uppercase tracking-[0.2em] text-slate-300">
                Tray Quick View
              </h2>
              <p class="mt-1 text-xs text-slate-500">Compact view for the tray popover.</p>
            </div>
            <span class="rounded-full border border-slate-700/80 px-3 py-1 text-xs text-slate-400">
              {traySessions.length} sessions
            </span>
          </div>
          <div class="mt-4 space-y-2 text-sm">
            {#each traySessions as session (session.sessionUid)}
              <div class="flex items-center justify-between rounded-lg border border-slate-800/80 bg-slate-950/60 px-3 py-2">
                <div>
                  <p class="font-semibold text-slate-100">{session.name}</p>
                  <p class="text-xs text-slate-500">{session.sessionUid.slice(0, 8)}</p>
                </div>
                <span class="text-xs text-slate-400">{session.status}</span>
              </div>
            {/each}
          </div>
        </div>
        <SessionList query={normalizedQuery} dense />
      </section>
    {:else}
      <section class="mt-10 grid gap-6 lg:grid-cols-[minmax(0,1.4fr)_minmax(0,0.9fr)]">
        <div class="space-y-6">
          <div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5">
            <div class="flex items-start justify-between gap-4">
              <div>
                <h2 class="text-sm font-semibold uppercase tracking-[0.2em] text-slate-300">Sessions</h2>
                <p class="mt-1 text-xs text-slate-500">Click to expand and drill into panes.</p>
              </div>
              <span class="rounded-full border border-slate-700/80 px-3 py-1 text-xs text-slate-400">
                {$sessions.length} total
              </span>
            </div>
            <div class="mt-4">
              <SessionList query={normalizedQuery} />
            </div>
          </div>

          <ActivityGraph height={160} />
        </div>

        <div class="space-y-6">
          <div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5">
            <div class="flex items-start justify-between gap-4">
              <div>
                <h2 class="text-sm font-semibold uppercase tracking-[0.2em] text-slate-300">
                  Tray Preview
                </h2>
                <p class="mt-1 text-xs text-slate-500">What the tray popover should surface.</p>
              </div>
              <span class="rounded-full border border-slate-700/80 px-3 py-1 text-xs text-slate-400">
                {pendingEscalations.length} alerts
              </span>
            </div>
            <div class="mt-4 space-y-2">
              {#each traySessions as session (session.sessionUid)}
                <div class="flex items-center justify-between rounded-lg border border-slate-800/80 bg-slate-950/60 px-3 py-2 text-sm">
                  <div>
                    <p class="font-semibold text-slate-100">{session.name}</p>
                    <p class="text-xs text-slate-500">{session.sessionUid.slice(0, 8)}</p>
                  </div>
                  <span class="text-xs text-slate-400">{session.status}</span>
                </div>
              {/each}
              {#if traySessions.length === 0}
                <p class="text-xs text-slate-500">No sessions reported yet.</p>
              {/if}
            </div>
            <div class="mt-4 grid gap-2 text-xs text-slate-400">
              <div class="flex items-center justify-between rounded-lg border border-slate-800/80 bg-slate-950/40 px-3 py-2">
                <span>Quiet hours</span>
                <span class="text-slate-300">{formatQuietHours($settings.quietHoursStart, $settings.quietHoursEnd)}</span>
              </div>
              <div class="flex items-center justify-between rounded-lg border border-slate-800/80 bg-slate-950/40 px-3 py-2">
                <span>Notifications</span>
                <span class="text-slate-300">{$settings.showNotifications ? 'Enabled' : 'Muted'}</span>
              </div>
            </div>
          </div>

          {#if $selectedSession}
            <div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5">
              <div class="flex items-start justify-between gap-4">
                <div>
                  <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Session Focus</p>
                  <p class="mt-2 text-xl font-semibold text-white">{$selectedSession.name}</p>
                  <p class="mt-1 text-xs text-slate-500">{$selectedSession.sessionUid}</p>
                </div>
                <button
                  class="rounded-lg border border-slate-700/80 bg-slate-950 px-3 py-2 text-xs text-slate-300 transition hover:border-slate-500"
                  type="button"
                  on:click={() => selectSession(null)}
                >
                  Clear focus
                </button>
              </div>
              <PaneList
                panes={$selectedSession.panes ?? []}
                selectable
                selectedPaneId={selectedPaneId}
                on:select={(event) => (selectedPaneId = event.detail.paneUid)}
              />
              <div class="mt-4">
                <OutputPreview paneId={selectedPaneId} />
              </div>
            </div>
          {:else}
            <div class="rounded-2xl border border-dashed border-slate-800 bg-slate-900/40 p-6 text-sm text-slate-500">
              Select a session to inspect pane details and live output.
            </div>
          {/if}

          <EscalationPanel />
          <TimelinePanel />
        </div>
      </section>
    {/if}
  </div>
</main>
