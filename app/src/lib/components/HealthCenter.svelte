<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { goto } from '$app/navigation';
  import { save } from '@tauri-apps/plugin-dialog';
  import { open as openExternal } from '@tauri-apps/plugin-shell';
  import { daemonRestart, exportDiagnostics, rpcCallWithRetry } from '$lib/tauri';
  import { connectionState, lastConnectionError, lastHealthCheck } from '$lib/stores/connection';
  import { settings } from '$lib/stores/settings';
  import { getConnectionStatus } from '$lib/status';

  export let open = false;

  const dispatch = createEventDispatcher<{ close: void }>();

  let helloLoading = false;
  let helloError: string | null = null;
  let daemonVersion: string | null = null;
  let protocolVersion: number | null = null;
  let schemaVersion: number | null = null;
  let capabilities: string[] = [];
  let openHandled = false;

  let actionStatus: string | null = null;
  let actionError: string | null = null;
  let restartLoading = false;

  const status = () => getConnectionStatus($connectionState);

  const formatTimestamp = (date: Date | null) => {
    if (!date) return 'Never';
    return date.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  };

  const isVersionMismatch = (message: string) =>
    /incompatible|protocolversion=|schemaversion=/i.test(message);

  const isWslMissing = (message: string) =>
    /wsl.*(missing|not installed|not found)|no\s+wsl/i.test(message);

  const isDaemonStopped = (message: string) =>
    /daemon.*(not running|stopped|not\s+running|failed health check)/i.test(message);

  const guidance = (message: string | null) => {
    if (!message) return null;
    if (isVersionMismatch(message)) {
      return {
        title: 'Version mismatch detected',
        detail: 'The daemon protocol/schema version differs from the app. Run the upgrade flow to sync versions.'
      };
    }
    if (isWslMissing(message)) {
      return {
        title: 'WSL not detected',
        detail: 'Install WSL and reboot, then re-run the setup wizard to pick a distro.'
      };
    }
    if (isDaemonStopped(message)) {
      return {
        title: 'Daemon appears offline',
        detail: 'Restart the daemon from the actions below or re-run the setup wizard.'
      };
    }
    return {
      title: 'Connection issue detected',
      detail: message
    };
  };

  const loadHello = async () => {
    helloLoading = true;
    helloError = null;
    try {
      const response = await rpcCallWithRetry<Record<string, unknown>>('core.hello');
      daemonVersion = (response.daemonVersion as string | undefined) ?? null;
      protocolVersion = (response.protocolVersion as number | undefined) ?? null;
      schemaVersion = (response.schemaVersion as number | undefined) ?? null;
      const caps = response.capabilities as Record<string, boolean> | undefined;
      if (caps) {
        capabilities = Object.entries(caps)
          .filter(([, enabled]) => enabled)
          .map(([key]) => key.toUpperCase());
      } else {
        capabilities = [];
      }
    } catch (error) {
      helloError = error instanceof Error ? error.message : error ? String(error) : 'Unable to fetch daemon info';
    } finally {
      helloLoading = false;
    }
  };

  const handleRestart = async () => {
    restartLoading = true;
    actionError = null;
    actionStatus = null;
    try {
      await daemonRestart();
      actionStatus = 'Daemon restart triggered.';
      await loadHello();
    } catch (error) {
      actionError = error instanceof Error ? error.message : error ? String(error) : 'Failed to restart daemon';
    } finally {
      restartLoading = false;
    }
  };

  const handleExport = async () => {
    actionError = null;
    actionStatus = null;
    try {
      const path = await save({
        defaultPath: 'ntm-tracker-diagnostics',
        title: 'Export diagnostics'
      });
      if (!path) return;
      await exportDiagnostics(path);
      actionStatus = `Diagnostics exported to ${path}`;
    } catch (error) {
      actionError = error instanceof Error ? error.message : error ? String(error) : 'Export failed';
    }
  };

  const handleTroubleshoot = async () => {
    actionError = null;
    actionStatus = null;
    try {
      await openExternal('https://github.com/Al2800/ntm-tracker-2/tree/main/docs');
      actionStatus = 'Opened troubleshooting docs in your browser.';
    } catch (error) {
      actionError = error instanceof Error ? error.message : error ? String(error) : 'Unable to open docs';
    }
  };

  const handleCopyError = async () => {
    if (!$lastConnectionError) return;
    actionError = null;
    actionStatus = null;
    try {
      await navigator.clipboard.writeText($lastConnectionError);
      actionStatus = 'Error copied to clipboard.';
    } catch (error) {
      actionError = error instanceof Error ? error.message : error ? String(error) : 'Unable to copy error';
    }
  };

  const handleWizard = async () => {
    dispatch('close');
    await goto('/wizard');
  };

  const close = () => dispatch('close');

  $: if (open && !openHandled) {
    openHandled = true;
    void loadHello();
  }

  $: if (!open) {
    openHandled = false;
    actionStatus = null;
    actionError = null;
  }

  $: statusBadge = status();
  $: guide = guidance($lastConnectionError ?? null);
</script>

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4" role="dialog" aria-modal="true">
    <button class="absolute inset-0 bg-slate-950/70" on:click={close} aria-label="Close health center"></button>

    <div class="relative w-full max-w-4xl card-lg space-y-6">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <p class="label">Health Center</p>
          <h2 class="mt-2 text-2xl font-semibold text-text-primary">Connection &amp; recovery</h2>
          <p class="mt-2 text-sm text-text-secondary">
            Diagnose daemon/WSL issues and run recovery actions without leaving the app.
          </p>
        </div>
        <button class="btn btn-ghost btn-sm" type="button" on:click={close}>
          Close
        </button>
      </div>

      <div class="grid gap-4 lg:grid-cols-2">
        <div class="card-compact space-y-3">
          <div class="flex items-center justify-between">
            <span class="label">Status</span>
            <span class="badge {statusBadge.badge}">{statusBadge.label}</span>
          </div>
          <div class="text-sm text-text-secondary space-y-1">
            <p><span class="label-sm">Transport</span> <span class="ml-2">{$settings.transport}</span></p>
            <p><span class="label-sm">WSL distro</span> <span class="ml-2">{$settings.wslDistro ?? 'default'}</span></p>
            <p><span class="label-sm">Last check</span> <span class="ml-2">{formatTimestamp($lastHealthCheck)}</span></p>
          </div>
          {#if $lastConnectionError}
            <div class="rounded-lg border border-border bg-surface-base px-3 py-2 text-xs text-text-secondary">
              {$lastConnectionError}
            </div>
          {/if}
        </div>

        <div class="card-compact space-y-3">
          <div class="flex items-center justify-between">
            <span class="label">Daemon details</span>
            <button class="btn btn-ghost btn-sm" type="button" on:click={loadHello} disabled={helloLoading}>
              Refresh
            </button>
          </div>
          {#if helloLoading}
            <p class="text-sm text-text-subtle">Loading daemon details…</p>
          {:else if helloError}
            <p class="text-sm text-status-error-text">{helloError}</p>
          {:else}
            <div class="text-sm text-text-secondary space-y-1">
              <p><span class="label-sm">Version</span> <span class="ml-2">{daemonVersion ?? 'unknown'}</span></p>
              <p><span class="label-sm">Protocol</span> <span class="ml-2">{protocolVersion ?? '—'}</span></p>
              <p><span class="label-sm">Schema</span> <span class="ml-2">{schemaVersion ?? '—'}</span></p>
              <p><span class="label-sm">Capabilities</span> <span class="ml-2">{capabilities.join(', ') || '—'}</span></p>
            </div>
          {/if}
        </div>
      </div>

      {#if guide}
        <div class="card card-warning">
          <h3 class="text-sm font-semibold text-status-warning-text">{guide.title}</h3>
          <p class="mt-1 text-sm text-status-warning-text/90">{guide.detail}</p>
        </div>
      {/if}

      <div class="space-y-3">
        <h3 class="label">Recovery actions</h3>
        <div class="flex flex-wrap gap-3">
          <button class="btn btn-primary" type="button" on:click={handleRestart} disabled={restartLoading}>
            {#if restartLoading}Restarting…{:else}Restart daemon{/if}
          </button>
          <button class="btn btn-secondary" type="button" on:click={handleWizard}>
            Re-run wizard
          </button>
          <button class="btn btn-secondary" type="button" on:click={handleExport}>
            Export diagnostics
          </button>
          <button class="btn btn-secondary" type="button" on:click={handleTroubleshoot}>
            Open troubleshooting
          </button>
          {#if $lastConnectionError}
            <button class="btn btn-ghost" type="button" on:click={handleCopyError}>
              Copy last error
            </button>
          {/if}
        </div>
        {#if actionStatus}
          <p class="text-xs text-text-secondary">{actionStatus}</p>
        {/if}
        {#if actionError}
          <p class="text-xs text-status-error-text">{actionError}</p>
        {/if}
      </div>
    </div>
  </div>
{/if}
