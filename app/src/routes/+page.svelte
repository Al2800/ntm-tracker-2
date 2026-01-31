<svelte:head>
  <title>NTM Tracker</title>
</svelte:head>

<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { connectionState, lastConnectionError } from '$lib/stores/connection';
  import { events } from '$lib/stores/events';
  import {
    sessions,
    selectedSession,
    selectSession,
    pinnedSessionIds,
    mutedSessionIds,
    togglePinSession,
    toggleMuteSession
  } from '$lib/stores/sessions';
  import { settings, updateSettings } from '$lib/stores/settings';
  import { getConnectionStatus, getSessionStatus, sortBySessionStatus } from '$lib/status';
  import { CommandBar, DashboardLayout, Sidebar } from '$lib/components/layout';
  import OverviewCards from '$lib/components/OverviewCards.svelte';
  import SessionsHub from '$lib/components/SessionsHub.svelte';
  import InsightsPanel from '$lib/components/InsightsPanel.svelte';
  import PaneList from '$lib/components/PaneList.svelte';
  import OutputPreview from '$lib/components/OutputPreview.svelte';
  import EmptyState from '$lib/components/states/EmptyState.svelte';
  import HealthCenter from '$lib/components/HealthCenter.svelte';
  import { daemonRestart, getAttachCommand, rpcCallWithRetry } from '$lib/tauri';
  import type { Session } from '$lib/types';

  let query = '';
  let selectedPaneId: string | null = null;
  let lastSelectedSessionId: string | null = null;
  let healthCenterOpen = false;
  let actionNotice = '';
  let actionNoticeTimeout: ReturnType<typeof setTimeout> | null = null;
  let reconnecting = false;

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

  const announceAction = (message: string) => {
    actionNotice = message;
    if (actionNoticeTimeout) clearTimeout(actionNoticeTimeout);
    actionNoticeTimeout = setTimeout(() => {
      actionNotice = '';
    }, 3000);
  };

  const triggerReconnect = async () => {
    if (reconnecting) return;
    reconnecting = true;
    try {
      await daemonRestart();
      announceAction('Restarting daemon...');
    } catch (error) {
      const message =
        error instanceof Error ? error.message : error ? String(error) : 'Unable to restart daemon';
      announceAction(message);
    } finally {
      reconnecting = false;
    }
  };

  const resolveAttachTarget = (session: Session) => {
    const paneTarget = session.panes?.find((pane) => pane.tmuxPaneId)?.tmuxPaneId ?? null;
    return paneTarget ?? session.tmuxSessionId ?? session.name;
  };

  const copyAttachCommand = async (session: Session) => {
    const target = resolveAttachTarget(session);
    if (!target) {
      announceAction('Attach command unavailable for this session.');
      return;
    }
    try {
      const command = await getAttachCommand(target);
      await navigator.clipboard.writeText(command);
      announceAction('Attach command copied to clipboard.');
    } catch {
      try {
        await navigator.clipboard.writeText(`tmux attach -t ${target}`);
        announceAction('Attach command copied to clipboard.');
      } catch {
        announceAction('Unable to copy attach command.');
      }
    }
  };

  const killSession = async (session: Session) => {
    const confirmed = window.confirm(`Kill session "${session.name}"?`);
    if (!confirmed) return;
    try {
      await rpcCallWithRetry('actions.sessionKill', { sessionId: session.sessionId }, { idempotent: false });
      announceAction(`Killed session ${session.name}.`);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : error ? String(error) : 'Kill failed';
      announceAction(`Unable to kill session: ${message}`);
    }
  };

  const handleSessionAction = async (session: Session, action: string) => {
    switch (action) {
      case 'attach':
        await copyAttachCommand(session);
        break;
      case 'pin':
        {
          const wasPinned = $pinnedSessionIds.has(session.sessionId);
          togglePinSession(session.sessionId);
          announceAction(wasPinned ? 'Session unpinned.' : 'Session pinned.');
        }
        break;
      case 'mute':
        {
          const wasMuted = $mutedSessionIds.has(session.sessionId);
          toggleMuteSession(session.sessionId);
          announceAction(wasMuted ? 'Alerts unmuted.' : 'Alerts muted.');
        }
        break;
      case 'kill':
        await killSession(session);
        break;
      default:
        break;
    }
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
    <CommandBar
      bind:searchValue={query}
      focusSearch={focusRequested}
      on:openHealth={() => (healthCenterOpen = true)}
    />
  </svelte:fragment>

  <svelte:fragment slot="sidebar">
    <Sidebar title="Sessions" subtitle="Filter and browse active sessions" count={$sessions.length}>
      <SessionsHub
        searchQuery={query}
        on:action={(event) => handleSessionAction(event.detail.session, event.detail.action)}
      />
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
      {#if $connectionState !== 'connected'}
        <div
          class="card border-status-warning-ring bg-status-warning-muted text-status-warning-text"
          role="status"
          aria-live="polite"
        >
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div class="space-y-1">
              <p class="label">Connection</p>
              <p class="text-sm font-medium">{connectionStatus.label}</p>
              <p class="text-xs text-status-warning-text/80">{connectionStatus.description}</p>
              {#if $lastConnectionError}
                <p class="text-xs text-status-warning-text/70">
                  {$lastConnectionError}
                </p>
              {/if}
            </div>
            <div class="flex flex-wrap items-center gap-2">
              <button
                class="btn btn-sm btn-secondary"
                type="button"
                on:click={triggerReconnect}
                disabled={reconnecting}
                aria-busy={reconnecting}
              >
                {reconnecting ? 'Restarting…' : 'Restart daemon'}
              </button>
              <button
                class="btn btn-sm btn-ghost"
                type="button"
                on:click={() => (healthCenterOpen = true)}
              >
                Open Health Center
              </button>
            </div>
          </div>
        </div>
      {/if}
      <OverviewCards />
      {#if actionNotice}
        <div class="card border-status-info-ring bg-status-info-muted text-status-info-text text-sm" role="status" aria-live="polite">
          {actionNotice}
        </div>
      {/if}

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
            <div class="flex flex-wrap items-center gap-2">
              <button
                class="btn btn-secondary btn-sm"
                type="button"
                on:click={() => handleSessionAction($selectedSession, 'attach')}
                aria-label="Copy attach command"
              >
                Attach
              </button>
              <button
                class="btn btn-secondary btn-sm"
                type="button"
                on:click={() => handleSessionAction($selectedSession, 'pin')}
                aria-label={$pinnedSessionIds.has($selectedSession.sessionId) ? 'Unpin session' : 'Pin session'}
              >
                {$pinnedSessionIds.has($selectedSession.sessionId) ? 'Unpin' : 'Pin'}
              </button>
              <button
                class="btn btn-secondary btn-sm"
                type="button"
                on:click={() => handleSessionAction($selectedSession, 'mute')}
                aria-label={$mutedSessionIds.has($selectedSession.sessionId) ? 'Unmute alerts' : 'Mute alerts'}
              >
                {$mutedSessionIds.has($selectedSession.sessionId) ? 'Unmute' : 'Mute'}
              </button>
              <button
                class="btn btn-secondary btn-sm text-status-error-text"
                type="button"
                on:click={() => handleSessionAction($selectedSession, 'kill')}
                aria-label="Kill session"
              >
                Kill
              </button>
              <button
                class="btn btn-secondary btn-sm"
                type="button"
                on:click={() => selectSession(null)}
                aria-label="Clear session focus"
              >
                Clear focus
              </button>
            </div>
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
              <div class="tray-item-compact flex items-center gap-2">
                <button
                  type="button"
                  class="min-w-0 flex-1 text-left"
                  on:click={() => openDashboardWithSession(session.sessionId)}
                  aria-label="Open {session.name} in dashboard"
                >
                  <p class="truncate text-sm font-medium text-text-primary">{session.name}</p>
                  <span class="mt-0.5 inline-flex items-center gap-1 text-[10px] text-text-muted" aria-label="Status: {sessionStatus.label}">
                    <span class="status-dot {sessionStatus.dot}" aria-hidden="true"></span>
                    {sessionStatus.label}
                  </span>
                </button>
                <div class="flex items-center gap-1">
                  <button
                    type="button"
                    class="rounded bg-surface-base/60 px-1.5 py-1 text-[10px] text-text-muted hover:text-text-primary focus-ring"
                    title="Copy attach command"
                    aria-label="Copy attach command"
                    on:click|stopPropagation={() => handleSessionAction(session, 'attach')}
                  >
                    ⧉
                  </button>
                  <button
                    type="button"
                    class={`rounded bg-surface-base/60 px-1.5 py-1 text-[10px] focus-ring ${
                      $pinnedSessionIds.has(session.sessionId) ? 'text-accent' : 'text-text-muted hover:text-text-primary'
                    }`}
                    title={$pinnedSessionIds.has(session.sessionId) ? 'Unpin session' : 'Pin session'}
                    aria-label={$pinnedSessionIds.has(session.sessionId) ? 'Unpin session' : 'Pin session'}
                    on:click|stopPropagation={() => handleSessionAction(session, 'pin')}
                  >
                    📌
                  </button>
                  <button
                    type="button"
                    class={`rounded bg-surface-base/60 px-1.5 py-1 text-[10px] focus-ring ${
                      $mutedSessionIds.has(session.sessionId)
                        ? 'text-status-warning-text'
                        : 'text-text-muted hover:text-text-primary'
                    }`}
                    title={$mutedSessionIds.has(session.sessionId) ? 'Unmute alerts' : 'Mute alerts'}
                    aria-label={$mutedSessionIds.has(session.sessionId) ? 'Unmute alerts' : 'Mute alerts'}
                    on:click|stopPropagation={() => handleSessionAction(session, 'mute')}
                  >
                    {$mutedSessionIds.has(session.sessionId) ? '🔕' : '🔔'}
                  </button>
                </div>
              </div>
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

<HealthCenter open={healthCenterOpen} on:close={() => (healthCenterOpen = false)} />
