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

  /**
   * Navigate from tray popover to main dashboard with a specific session selected.
   */
  const openDashboardWithSession = async (sessionUid: string) => {
    selectSession(sessionUid);
    try {
      const { getCurrentWindow, Window } = await import('@tauri-apps/api/window');
      await getCurrentWindow().hide();
      const main = await Window.getByLabel('main');
      if (main) {
        await main.show();
        await main.setFocus();
      }
    } catch {
      goto('/?focusSession=' + sessionUid);
    }
  };

  /**
   * Navigate from tray popover to main dashboard scrolled to escalations.
   */
  const openDashboardWithEscalation = async () => {
    try {
      const { getCurrentWindow, Window } = await import('@tauri-apps/api/window');
      await getCurrentWindow().hide();
      const main = await Window.getByLabel('main');
      if (main) {
        await main.show();
        await main.setFocus();
        await main.emit('scroll-to-escalations');
      }
    } catch {
      goto('/?scrollTo=escalations');
    }
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

<main
  class={`min-h-screen bg-slate-950 text-slate-100 ${compactMode ? 'popover-mode' : ''}`}
  aria-label={compactMode ? 'NTM Tracker compact view' : 'NTM Tracker dashboard'}
>
  <!-- Skip link for keyboard navigation -->
  <a href="#main-content" class="skip-link">Skip to main content</a>

  <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,_rgba(56,189,248,0.16),_rgba(15,23,42,0.2),_transparent_65%)]" aria-hidden="true"></div>
  <div id="main-content" class={`relative mx-auto ${compactMode ? 'max-w-full px-3 py-4' : 'max-w-6xl px-6 py-12'}`}>
    {#if !compactMode}
      <header class="flex flex-wrap items-center justify-between gap-6">
        <div>
          <p class="text-xs uppercase tracking-[0.4em] text-slate-400" aria-hidden="true">NTM Tracker</p>
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

      <nav class="mt-10 flex flex-wrap items-center gap-4 rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4" aria-label="Dashboard controls">
        <div class="flex min-w-[240px] flex-1 items-center gap-3">
          <label for="session-search" class="text-xs uppercase tracking-[0.3em] text-slate-500">Search</label>
          <input
            id="session-search"
            class="input"
            placeholder="Find a session or UID… (Ctrl+K)"
            aria-describedby="search-hint"
            bind:value={query}
            bind:this={searchInput}
          />
          <span id="search-hint" class="sr-only">Press Control+K to focus search</span>
        </div>
        <div class="flex flex-wrap items-center gap-2 text-xs text-slate-300" role="group" aria-label="Quick actions">
          <button
            class="btn btn-secondary focus-ring"
            type="button"
            on:click={() => goto('/settings')}
            aria-label="Open settings"
          >
            Settings
          </button>
          <button
            class={`btn focus-ring ${
              $settings.showNotifications
                ? 'border-emerald-500/60 bg-emerald-500/10 text-emerald-100'
                : 'btn-secondary text-slate-400'
            }`}
            type="button"
            on:click={toggleNotifications}
            aria-pressed={$settings.showNotifications}
            aria-label={$settings.showNotifications ? 'Mute notifications' : 'Enable notifications'}
          >
            {$settings.showNotifications ? 'Notifications on' : 'Notifications muted'}
          </button>
          <button
            class="btn btn-secondary focus-ring"
            type="button"
            on:click={() => goto('/wizard')}
            aria-label="Open setup wizard"
          >
            Setup wizard
          </button>
        </div>
      </nav>

      <section class="mt-10" aria-label="Overview statistics">
        <OverviewCards />
      </section>
    {:else}
      <!-- Compact mode header: minimal status bar -->
      <header class="tray-item">
        <span class="label-sm">NTM Tracker</span>
        <span class="badge {connectionStatus.badge} text-[10px] px-2 py-0.5" role="status" aria-live="polite">
          {connectionStatus.label}
        </span>
      </header>
    {/if}

    {#if compactMode}
      <section class="mt-3 space-y-3" aria-label="Quick overview">
        <!-- Compact overview stats -->
        <div class="grid grid-cols-3 gap-2 text-center" role="group" aria-label="Statistics summary">
          <div class="card-compact px-2 py-2" role="status">
            <p class="text-lg font-bold text-text-primary" aria-label="{$sessions.length} sessions">{$sessions.length}</p>
            <p class="label-sm">Sessions</p>
          </div>
          <div class="card-compact px-2 py-2" role="status">
            <p class="text-lg font-bold text-status-success" aria-label="{sortedSessions.filter(s => s.status === 'active').length} active">{sortedSessions.filter(s => s.status === 'active').length}</p>
            <p class="label-sm">Active</p>
          </div>
          <div class="card-compact px-2 py-2" role="status">
            <p class="text-lg font-bold text-status-warning" aria-label="{pendingEscalations.length} alerts">{pendingEscalations.length}</p>
            <p class="label-sm">Alerts</p>
          </div>
        </div>

        <!-- Session list (compact) -->
        <nav class="card-compact p-3" aria-label="Sessions">
          <div class="flex items-center justify-between mb-2">
            <span class="label-sm" id="sessions-heading">Sessions</span>
            <button
              class="text-[10px] text-accent hover:text-accent-hover transition focus-ring rounded"
              type="button"
              aria-label="Open full dashboard"
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
          <ul class="space-y-1.5 max-h-[280px] overflow-y-auto" aria-labelledby="sessions-heading" role="list">
            {#each sortedSessions.slice(0, 8) as session (session.sessionUid)}
              {@const sessionStatus = getSessionStatus(session.status)}
              <li role="listitem">
                <button
                  type="button"
                  class="tray-item-compact w-full cursor-pointer hover:border-border-strong focus-ring"
                  on:click={() => openDashboardWithSession(session.sessionUid)}
                  aria-label="Open {session.name} in dashboard"
                >
                  <div class="min-w-0 flex-1 text-left">
                    <p class="text-sm font-medium text-text-primary truncate">{session.name}</p>
                  </div>
                  <span class="ml-2 shrink-0 flex items-center gap-1" aria-label="Status: {sessionStatus.label}">
                    <span class="status-dot {sessionStatus.dot}" aria-hidden="true"></span>
                    <span class="text-[10px] text-text-muted">{sessionStatus.label}</span>
                  </span>
                </button>
              </li>
            {/each}
            {#if sortedSessions.length === 0}
              <li class="text-xs text-text-subtle text-center py-4" role="listitem">No sessions yet</li>
            {/if}
            {#if sortedSessions.length > 8}
              <li class="text-[10px] text-text-subtle text-center pt-1" role="listitem">+{sortedSessions.length - 8} more</li>
            {/if}
          </ul>
        </nav>

        <!-- Pending alerts (compact) -->
        {#if pendingEscalations.length > 0}
          <aside class="card-compact card-warning p-3" role="alert" aria-label="Pending alerts">
            <div class="flex items-center justify-between mb-2">
              <span class="label-sm text-status-warning-text">Pending Alerts</span>
              <button
                type="button"
                class="text-[10px] text-status-warning-text hover:text-status-warning transition focus-ring rounded"
                on:click={openDashboardWithEscalation}
                aria-label="View all alerts in dashboard"
              >
                View All →
              </button>
            </div>
            <ul class="space-y-1" role="list">
              {#each pendingEscalations.slice(0, 3) as escalation (escalation.id)}
                <li role="listitem">
                  <button
                    type="button"
                    class="tray-item-compact w-full bg-surface-base text-status-warning-text cursor-pointer hover:border-status-warning-ring focus-ring text-left"
                    on:click={openDashboardWithEscalation}
                    aria-label="View alert: {escalation.message || 'Attention required'}"
                  >
                    {escalation.message || 'Attention required'}
                  </button>
                </li>
              {/each}
              {#if pendingEscalations.length > 3}
                <li class="text-[10px] text-status-warning-text/70 text-center" role="listitem">+{pendingEscalations.length - 3} more</li>
              {/if}
            </ul>
          </aside>
        {/if}

        <!-- Quick actions -->
        <div class="flex gap-2" role="group" aria-label="Quick actions">
          <button
            class="btn btn-sm btn-secondary flex-1 focus-ring"
            type="button"
            on:click={toggleNotifications}
            aria-pressed={$settings.showNotifications}
            aria-label={$settings.showNotifications ? 'Mute notifications' : 'Enable notifications'}
          >
            {$settings.showNotifications ? '🔔 Mute' : '🔕 Unmute'}
          </button>
          <button
            class="btn btn-sm btn-secondary flex-1 focus-ring"
            type="button"
            on:click={() => goto('/settings')}
            aria-label="Open settings"
          >
            <span aria-hidden="true">⚙</span> Settings
          </button>
        </div>
      </section>
    {:else}
      <section class="mt-10 grid gap-6 lg:grid-cols-[minmax(0,1.4fr)_minmax(0,0.9fr)]" aria-label="Dashboard content">
        <div class="space-y-6">
          <article class="card" aria-labelledby="sessions-section-title">
            <div class="flex items-start justify-between gap-4">
              <div>
                <h2 id="sessions-section-title" class="label">Sessions</h2>
                <p class="mt-1 text-xs text-text-subtle">Click to expand and drill into panes.</p>
              </div>
              <span class="badge badge-neutral" role="status">
                {$sessions.length} total
              </span>
            </div>
            <div class="mt-4">
              <SessionList query={normalizedQuery} />
            </div>
          </article>

          <ActivityGraph height={160} />
        </div>

        <aside class="space-y-6" aria-label="Sidebar panels">
          <article class="card" aria-labelledby="tray-preview-title">
            <div class="flex items-start justify-between gap-4">
              <div>
                <h2 id="tray-preview-title" class="label">Tray Preview</h2>
                <p class="mt-1 text-xs text-text-subtle">What the tray popover should surface.</p>
              </div>
              <span class="badge {pendingEscalations.length > 0 ? 'badge-warning' : 'badge-neutral'}" role="status">
                {pendingEscalations.length} alerts
              </span>
            </div>
            <ul class="mt-4 space-y-2" role="list" aria-label="Session preview">
              {#each traySessions as session (session.sessionUid)}
                <li class="tray-item" role="listitem">
                  <div>
                    <p class="font-semibold text-text-primary">{session.name}</p>
                    <p class="text-xs text-text-muted font-mono">{session.sessionUid.slice(0, 8)}</p>
                  </div>
                  <span class="text-xs text-text-secondary">{session.status}</span>
                </li>
              {/each}
              {#if traySessions.length === 0}
                <li class="text-xs text-text-subtle" role="listitem">No sessions reported yet.</li>
              {/if}
            </ul>
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
          </article>

          {#if $selectedSession}
            {@const sessionStatus = getSessionStatus($selectedSession.status)}
            <article class="card" aria-labelledby="focus-panel-title" role="region">
              <!-- Session header with status and metrics -->
              <div class="flex items-start justify-between gap-4">
                <div>
                  <p id="focus-panel-title" class="label">Session Focus</p>
                  <h3 class="mt-2 text-xl font-semibold text-text-primary">{$selectedSession.name}</h3>
                  <div class="mt-1 flex items-center gap-2">
                    <span class="badge {sessionStatus.badge}" role="status">
                      <span class="status-dot {sessionStatus.dot}" aria-hidden="true"></span>
                      {sessionStatus.label}
                    </span>
                    <span class="text-xs text-text-subtle font-mono" aria-label="Session ID">{$selectedSession.sessionUid.slice(0, 12)}</span>
                  </div>
                </div>
                <button
                  class="btn btn-sm btn-secondary focus-ring"
                  type="button"
                  on:click={() => selectSession(null)}
                  aria-label="Clear session focus"
                >
                  Clear focus
                </button>
              </div>

              <!-- Session metrics -->
              <div class="mt-4 grid grid-cols-3 gap-2 text-center" role="group" aria-label="Session metrics">
                <div class="rounded-lg border border-border bg-surface-base px-2 py-2" role="status">
                  <p class="text-lg font-bold text-text-primary">{$selectedSession.paneCount ?? ($selectedSession.panes?.length ?? 0)}</p>
                  <p class="label-sm">Panes</p>
                </div>
                <div class="rounded-lg border border-border bg-surface-base px-2 py-2" role="status">
                  <p class="text-lg font-bold text-status-success">{($selectedSession.panes ?? []).filter(p => p.status === 'active').length}</p>
                  <p class="label-sm">Active</p>
                </div>
                <div class="rounded-lg border border-border bg-surface-base px-2 py-2" role="status">
                  <p class="text-lg font-bold text-status-warning">{($selectedSession.panes ?? []).filter(p => p.status === 'waiting').length}</p>
                  <p class="label-sm">Waiting</p>
                </div>
              </div>

              <!-- Pane list -->
              <div class="mt-4" role="region" aria-labelledby="panes-label">
                <p id="panes-label" class="label mb-2">Panes</p>
                <PaneList
                  panes={$selectedSession.panes ?? []}
                  selectable
                  selectedPaneId={selectedPaneId}
                  on:select={(event) => (selectedPaneId = event.detail.paneUid)}
                />
              </div>

              <!-- Output preview -->
              <div class="mt-4" role="region" aria-labelledby="output-label">
                <p id="output-label" class="label mb-2">Output Preview</p>
                <OutputPreview paneId={selectedPaneId} />
              </div>
            </article>
          {:else}
            <div class="card border-dashed bg-surface-base/40 p-6" role="status">
              <p class="text-sm text-text-subtle text-center">
                Select a session to inspect pane details and live output.
              </p>
            </div>
          {/if}

          <EscalationPanel />
          <TimelinePanel />
        </aside>
      </section>
    {/if}
  </div>
</main>
