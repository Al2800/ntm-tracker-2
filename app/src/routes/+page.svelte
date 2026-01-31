<svelte:head>
  <title>NTM Tracker</title>
</svelte:head>

<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { connectionState } from '$lib/stores/connection';
  import { events } from '$lib/stores/events';
  import { sessions, selectedSession, selectSession } from '$lib/stores/sessions';
  import { settings, updateSettings } from '$lib/stores/settings';
  import { getConnectionStatus, getSessionStatus, sortBySessionStatus } from '$lib/status';
  import { CommandBar, DashboardLayout, Sidebar } from '$lib/components/layout';
  import OverviewCards from '$lib/components/OverviewCards.svelte';
  import SessionsHub from '$lib/components/SessionsHub.svelte';
  import InsightsPanel from '$lib/components/InsightsPanel.svelte';
  import PaneList from '$lib/components/PaneList.svelte';
  import OutputPreview from '$lib/components/OutputPreview.svelte';
  import EmptyState from '$lib/components/states/EmptyState.svelte';

  let query = '';
  let selectedPaneId: string | null = null;
  let lastSelectedSessionId: string | null = null;

  $: connectionStatus = getConnectionStatus($connectionState);
  $: focusRequested = $page.url.searchParams.get('focusSearch') === '1';
  $: compactMode =
    $page.url.searchParams.get('view') === 'compact' ||
    $page.url.searchParams.get('compact') === '1';

  $: if (($selectedSession?.sessionId ?? null) !== lastSelectedSessionId) {
    lastSelectedSessionId = $selectedSession?.sessionId ?? null;
    selectedPaneId = null;
  }

  $: sortedSessions = sortBySessionStatus($sessions).sort((a, b) => {
    const rankA = getSessionStatus(a.status).rank;
    const rankB = getSessionStatus(b.status).rank;
    if (rankA !== rankB) return 0;
    return a.name.localeCompare(b.name);
  });

  $: traySessions = sortedSessions.slice(0, 4);
  $: pendingEscalations = $events.filter(
    (event) => event.eventType === 'escalation' && (event.status ?? 'pending') === 'pending'
  );

  $: activeSessions = $sessions.filter((session) => session.status === 'active').length;
  $: idleSessions = $sessions.filter((session) => session.status === 'idle').length;
  $: waitingSessions = $sessions.filter((session) =>
    (session.panes ?? []).some((pane) => pane.status === 'waiting')
  ).length;

  const toggleNotifications = () => {
    updateSettings({ showNotifications: !$settings.showNotifications });
  };

  /**
   * Navigate from tray popover to main dashboard with a specific session selected.
   */
  const openDashboardWithSession = async (sessionId: string) => {
    selectSession(sessionId);
    try {
      const { getCurrentWindow, Window } = await import('@tauri-apps/api/window');
      await getCurrentWindow().hide();
      const main = await Window.getByLabel('main');
      if (main) {
        await main.show();
        await main.setFocus();
      }
    } catch {
      goto('/?focusSession=' + sessionId);
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
    const onKeydown = async (event: KeyboardEvent) => {
      if (event.key === 'Escape' && compactMode) {
        event.preventDefault();
        try {
          const { getCurrentWindow } = await import('@tauri-apps/api/window');
          await getCurrentWindow().hide();
        } catch {
          // Not in Tauri environment
        }
      }
    };

    window.addEventListener('keydown', onKeydown);
    return () => {
      window.removeEventListener('keydown', onKeydown);
    };
  });
</script>

<DashboardLayout>
  <svelte:fragment slot="command-bar">
    <CommandBar bind:searchValue={query} focusSearch={focusRequested} />
  </svelte:fragment>

  <svelte:fragment slot="sidebar">
    <Sidebar title="Sessions" subtitle="Filter and browse active sessions" count={$sessions.length}>
      <SessionsHub searchQuery={query} />
    </Sidebar>
  </svelte:fragment>

  <svelte:fragment slot="sidebar-footer">
    <div class="grid gap-2 text-xs">
      <div class="flex items-center justify-between rounded-lg border border-border bg-surface-base px-3 py-2">
        <span class="text-text-subtle">Active</span>
        <span class="text-text-primary">{activeSessions}</span>
      </div>
      <div class="flex items-center justify-between rounded-lg border border-border bg-surface-base px-3 py-2">
        <span class="text-text-subtle">Idle</span>
        <span class="text-text-primary">{idleSessions}</span>
      </div>
      <div class="flex items-center justify-between rounded-lg border border-border bg-surface-base px-3 py-2">
        <span class="text-text-subtle">Waiting</span>
        <span class="text-text-primary">{waitingSessions}</span>
      </div>
      <div class="flex items-center justify-between rounded-lg border border-border bg-surface-base px-3 py-2">
        <span class="text-text-subtle">Alerts</span>
        <span class="text-text-primary">{pendingEscalations.length}</span>
      </div>
    </div>
  </svelte:fragment>

  <svelte:fragment slot="focus">
    <div class="space-y-6">
      <OverviewCards />

      {#if $selectedSession}
        {@const sessionStatus = getSessionStatus($selectedSession.status)}
        <section class="card space-y-4" aria-label="Session focus">
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div>
              <p class="label">Session Focus</p>
              <h2 class="mt-2 text-2xl font-semibold text-text-primary">{$selectedSession.name}</h2>
              <div class="mt-1 flex items-center gap-2">
                <span class="badge {sessionStatus.badge}" role="status">
                  <span class="status-dot {sessionStatus.dot}" aria-hidden="true"></span>
                  {sessionStatus.label}
                </span>
                <span class="text-xs text-text-muted font-mono">{$selectedSession.sessionId.slice(0, 12)}</span>
              </div>
            </div>
            <button
              class="btn btn-secondary btn-sm"
              type="button"
              on:click={() => selectSession(null)}
              aria-label="Clear session focus"
            >
              Clear focus
            </button>
          </div>

          <div class="grid gap-2 sm:grid-cols-3" role="group" aria-label="Session metrics">
            <div class="rounded-lg border border-border bg-surface-base px-3 py-2 text-center">
              <p class="text-lg font-semibold text-text-primary">
                {$selectedSession.paneCount ?? ($selectedSession.panes?.length ?? 0)}
              </p>
              <p class="label-sm">Panes</p>
            </div>
            <div class="rounded-lg border border-border bg-surface-base px-3 py-2 text-center">
              <p class="text-lg font-semibold text-status-success">
                {($selectedSession.panes ?? []).filter((pane) => pane.status === 'active').length}
              </p>
              <p class="label-sm">Active</p>
            </div>
            <div class="rounded-lg border border-border bg-surface-base px-3 py-2 text-center">
              <p class="text-lg font-semibold text-status-warning">
                {($selectedSession.panes ?? []).filter((pane) => pane.status === 'waiting').length}
              </p>
              <p class="label-sm">Waiting</p>
            </div>
          </div>

          <div>
            <p class="label mb-2">Panes</p>
            <PaneList
              panes={$selectedSession.panes ?? []}
              selectable
              selectedPaneId={selectedPaneId}
              on:select={(event) => (selectedPaneId = event.detail.paneId)}
            />
          </div>

          <div>
            <p class="label mb-2">Output Preview</p>
            <OutputPreview paneId={selectedPaneId} />
          </div>
        </section>
      {:else}
        <div class="card border-dashed bg-surface-base/40">
          <EmptyState
            icon="sessions"
            title="Select a session"
            description="Pick a session from the sidebar to inspect panes and live output."
          />
        </div>
      {/if}
    </div>
  </svelte:fragment>

  <svelte:fragment slot="insights">
    <InsightsPanel />
  </svelte:fragment>

  <svelte:fragment slot="compact-header">
    <div class="flex items-center justify-between">
      <span class="label-sm">NTM Tracker</span>
      <span class="badge {connectionStatus.badge} text-[10px] px-2 py-0.5" role="status" aria-live="polite">
        {connectionStatus.label}
      </span>
    </div>
  </svelte:fragment>

  <svelte:fragment slot="compact-content">
    <section class="space-y-3" aria-label="Quick overview">
      <div class="grid grid-cols-3 gap-2 text-center" role="group" aria-label="Statistics summary">
        <div class="card-compact px-2 py-2" role="status">
          <p class="text-lg font-bold text-text-primary" aria-label="{$sessions.length} sessions">{$sessions.length}</p>
          <p class="label-sm">Sessions</p>
        </div>
        <div class="card-compact px-2 py-2" role="status">
          <p class="text-lg font-bold text-status-success" aria-label="{activeSessions} active">{activeSessions}</p>
          <p class="label-sm">Active</p>
        </div>
        <div class="card-compact px-2 py-2" role="status">
          <p class="text-lg font-bold text-status-warning" aria-label="{pendingEscalations.length} alerts">{pendingEscalations.length}</p>
          <p class="label-sm">Alerts</p>
        </div>
      </div>

      <nav class="card-compact p-3" aria-label="Sessions">
        <div class="mb-2 flex items-center justify-between">
          <span class="label-sm" id="sessions-heading">Sessions</span>
          <button
            class="rounded text-[10px] text-accent hover:text-accent-hover transition focus-ring"
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
        <ul class="max-h-[280px] space-y-1.5 overflow-y-auto" aria-labelledby="sessions-heading" role="list">
          {#each sortedSessions.slice(0, 8) as session (session.sessionId)}
            {@const sessionStatus = getSessionStatus(session.status)}
            <li role="listitem">
              <button
                type="button"
                class="tray-item-compact w-full cursor-pointer hover:border-border-strong focus-ring"
                on:click={() => openDashboardWithSession(session.sessionId)}
                aria-label="Open {session.name} in dashboard"
              >
                <div class="min-w-0 flex-1 text-left">
                  <p class="truncate text-sm font-medium text-text-primary">{session.name}</p>
                </div>
                <span class="ml-2 shrink-0 flex items-center gap-1" aria-label="Status: {sessionStatus.label}">
                  <span class="status-dot {sessionStatus.dot}" aria-hidden="true"></span>
                  <span class="text-[10px] text-text-muted">{sessionStatus.label}</span>
                </span>
              </button>
            </li>
          {/each}
          {#if sortedSessions.length === 0}
            <li class="py-4 text-center text-xs text-text-subtle" role="listitem">No sessions yet</li>
          {/if}
          {#if sortedSessions.length > 8}
            <li class="pt-1 text-center text-[10px] text-text-subtle" role="listitem">+{sortedSessions.length - 8} more</li>
          {/if}
        </ul>
      </nav>

      {#if pendingEscalations.length > 0}
        <aside class="card-compact card-warning p-3" role="alert" aria-label="Pending alerts">
          <div class="mb-2 flex items-center justify-between">
            <span class="label-sm text-status-warning-text">Pending Alerts</span>
            <button
              type="button"
              class="rounded text-[10px] text-status-warning-text hover:text-status-warning transition focus-ring"
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
                  class="tray-item-compact w-full cursor-pointer bg-surface-base text-status-warning-text hover:border-status-warning-ring focus-ring text-left"
                  on:click={openDashboardWithEscalation}
                  aria-label="View alert: {escalation.message || 'Attention required'}"
                >
                  {escalation.message || 'Attention required'}
                </button>
              </li>
            {/each}
            {#if pendingEscalations.length > 3}
              <li class="text-center text-[10px] text-status-warning-text/70" role="listitem">
                +{pendingEscalations.length - 3} more
              </li>
            {/if}
          </ul>
        </aside>
      {/if}
    </section>
  </svelte:fragment>

  <svelte:fragment slot="compact-footer">
    <div class="flex gap-2" role="group" aria-label="Quick actions">
      <button
        class="btn btn-sm btn-secondary flex-1"
        type="button"
        on:click={toggleNotifications}
        aria-pressed={$settings.showNotifications}
        aria-label={$settings.showNotifications ? 'Mute notifications' : 'Enable notifications'}
      >
        {$settings.showNotifications ? '🔔 Mute' : '🔕 Unmute'}
      </button>
      <button
        class="btn btn-sm btn-secondary flex-1"
        type="button"
        on:click={() => goto('/settings')}
        aria-label="Open settings"
      >
        <span aria-hidden="true">⚙</span> Settings
      </button>
    </div>
  </svelte:fragment>
</DashboardLayout>
