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
  import { getConnectionStatus, getSessionStatus, sortBySessionStatus } from '$lib/status';
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

  // Use centralized status system
  $: connectionStatus = getConnectionStatus($connectionState);

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

  $: sortedSessions = sortBySessionStatus($sessions).sort((a, b) => {
    // Secondary sort by name within same status
    const rankA = getSessionStatus(a.status).rank;
    const rankB = getSessionStatus(b.status).rank;
    if (rankA !== rankB) return 0; // Already sorted by status
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

<main class={`min-h-screen bg-slate-950 text-slate-100 ${compactMode ? 'popover-mode' : ''}`}>
  <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.16),_rgba(15,23,42,0.2),_transparent_65%)]"></div>
  <div class={`relative mx-auto ${compactMode ? 'max-w-full px-3 py-4' : 'max-w-6xl px-6 py-12'}`}>
    {#if !compactMode}
      <header class="flex flex-wrap items-center justify-between gap-6">
        <div>
          <p class="text-xs uppercase tracking-[0.4em] text-slate-400">NTM Tracker</p>
          <h1 class="mt-4 text-4xl font-semibold text-white">Session intelligence, at a glance.</h1>
          <p class="mt-3 max-w-2xl text-base text-slate-300">
            Monitor NTM sessions, compact events, and escalations with a crisp, always-on view — designed to
            feel right at home in your system tray.
          </p>
        </div>
        <div class="card-compact flex flex-col items-start gap-2">
          <span class="label">Connection</span>
          <span class="badge {connectionStatus.badge}">
            {connectionStatus.label}
          </span>
          {#if $lastConnectionError}
            <span class="text-xs text-text-subtle">{$lastConnectionError}</span>
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
    {:else}
      <!-- Compact mode header: minimal status bar -->
      <div class="tray-item">
        <span class="label-sm">NTM Tracker</span>
        <span class="badge {connectionStatus.badge} text-[10px] px-2 py-0.5">
          {connectionStatus.label}
        </span>
      </div>
    {/if}

    {#if compactMode}
      <section class="mt-3 space-y-3">
        <!-- Compact overview stats -->
        <div class="grid grid-cols-3 gap-2 text-center">
          <div class="card-compact px-2 py-2">
            <p class="text-lg font-bold text-text-primary">{$sessions.length}</p>
            <p class="label-sm">Sessions</p>
          </div>
          <div class="card-compact px-2 py-2">
            <p class="text-lg font-bold text-status-success">{sortedSessions.filter(s => s.status === 'active').length}</p>
            <p class="label-sm">Active</p>
          </div>
          <div class="card-compact px-2 py-2">
            <p class="text-lg font-bold text-status-warning">{pendingEscalations.length}</p>
            <p class="label-sm">Alerts</p>
          </div>
        </div>

        <!-- Session list (compact) -->
        <div class="card-compact p-3">
          <div class="flex items-center justify-between mb-2">
            <span class="label-sm">Sessions</span>
            <button
              class="text-[10px] text-accent hover:text-accent-hover transition"
              type="button"
              on:click={async () => {
                try {
                  const { getCurrentWindow, Window } = await import('@tauri-apps/api/window');
                  await getCurrentWindow().hide();
                  const main = await Window.getByLabel('main');
                  if (main) {
                    await main.show();
                    await main.setFocus();
                  }
                } catch {
                  goto('/');
                }
              }}
            >
              Open Dashboard →
            </button>
          </div>
          <div class="space-y-1.5 max-h-[280px] overflow-y-auto">
            {#each sortedSessions.slice(0, 8) as session (session.sessionUid)}
              {@const sessionStatus = getSessionStatus(session.status)}
              <div class="tray-item-compact">
                <div class="min-w-0 flex-1">
                  <p class="text-sm font-medium text-text-primary truncate">{session.name}</p>
                </div>
                <span class="ml-2 shrink-0 flex items-center gap-1">
                  <span class="status-dot {sessionStatus.dot}"></span>
                  <span class="text-[10px] text-text-muted">{sessionStatus.label}</span>
                </span>
              </div>
            {/each}
            {#if sortedSessions.length === 0}
              <p class="text-xs text-text-subtle text-center py-4">No sessions yet</p>
            {/if}
            {#if sortedSessions.length > 8}
              <p class="text-[10px] text-text-subtle text-center pt-1">+{sortedSessions.length - 8} more</p>
            {/if}
          </div>
        </div>

        <!-- Pending alerts (compact) -->
        {#if pendingEscalations.length > 0}
          <div class="card-compact card-critical p-3">
            <span class="label-sm text-status-warning-text">Pending Alerts</span>
            <div class="mt-2 space-y-1">
              {#each pendingEscalations.slice(0, 3) as escalation (escalation.id)}
                <div class="tray-item-compact bg-surface-base text-status-warning-text">
                  {escalation.message || 'Attention required'}
                </div>
              {/each}
              {#if pendingEscalations.length > 3}
                <p class="text-[10px] text-status-warning-text/70 text-center">+{pendingEscalations.length - 3} more</p>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Quick actions -->
        <div class="flex gap-2">
          <button
            class="btn btn-sm btn-secondary flex-1"
            type="button"
            on:click={toggleNotifications}
          >
            {$settings.showNotifications ? '🔔 Mute' : '🔕 Unmute'}
          </button>
          <button
            class="btn btn-sm btn-secondary flex-1"
            type="button"
            on:click={() => goto('/settings')}
          >
            ⚙ Settings
          </button>
        </div>
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
            {@const sessionStatus = getSessionStatus($selectedSession.status)}
            <div class="card">
              <!-- Session header with status and metrics -->
              <div class="flex items-start justify-between gap-4">
                <div>
                  <p class="label">Session Focus</p>
                  <p class="mt-2 text-xl font-semibold text-text-primary">{$selectedSession.name}</p>
                  <div class="mt-1 flex items-center gap-2">
                    <span class="badge {sessionStatus.badge}">
                      <span class="status-dot {sessionStatus.dot}"></span>
                      {sessionStatus.label}
                    </span>
                    <span class="text-xs text-text-subtle font-mono">{$selectedSession.sessionUid.slice(0, 12)}</span>
                  </div>
                </div>
                <button
                  class="btn btn-sm btn-secondary"
                  type="button"
                  on:click={() => selectSession(null)}
                >
                  Clear focus
                </button>
              </div>

              <!-- Session metrics -->
              <div class="mt-4 grid grid-cols-3 gap-2 text-center">
                <div class="rounded-lg border border-border bg-surface-base px-2 py-2">
                  <p class="text-lg font-bold text-text-primary">{$selectedSession.paneCount ?? ($selectedSession.panes?.length ?? 0)}</p>
                  <p class="label-sm">Panes</p>
                </div>
                <div class="rounded-lg border border-border bg-surface-base px-2 py-2">
                  <p class="text-lg font-bold text-status-success">{($selectedSession.panes ?? []).filter(p => p.status === 'active').length}</p>
                  <p class="label-sm">Active</p>
                </div>
                <div class="rounded-lg border border-border bg-surface-base px-2 py-2">
                  <p class="text-lg font-bold text-status-warning">{($selectedSession.panes ?? []).filter(p => p.status === 'waiting').length}</p>
                  <p class="label-sm">Waiting</p>
                </div>
              </div>

              <!-- Pane list -->
              <div class="mt-4">
                <p class="label mb-2">Panes</p>
                <PaneList
                  panes={$selectedSession.panes ?? []}
                  selectable
                  selectedPaneId={selectedPaneId}
                  on:select={(event) => (selectedPaneId = event.detail.paneUid)}
                />
              </div>

              <!-- Output preview -->
              <div class="mt-4">
                <p class="label mb-2">Output Preview</p>
                <OutputPreview paneId={selectedPaneId} />
              </div>
            </div>
          {:else}
            <div class="card border-dashed bg-surface-base/40 p-6">
              <p class="text-sm text-text-subtle text-center">
                Select a session to inspect pane details and live output.
              </p>
            </div>
          {/if}

          <EscalationPanel />
          <TimelinePanel />
        </div>
      </section>
    {/if}
  </div>
</main>
