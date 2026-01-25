<svelte:head>
  <title>Setup Wizard · NTM Tracker</title>
</svelte:head>

<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { daemonHealth, daemonStart, listWslDistros } from '$lib/tauri';
  import { saveSettingsNow, settings, updateSettings } from '$lib/stores/settings';

  type WizardStep = 'Welcome' | 'WSL' | 'Daemon' | 'Notifications' | 'Complete';
  const steps: WizardStep[] = ['Welcome', 'WSL', 'Daemon', 'Notifications', 'Complete'];

  let stepIndex = 0;

  let distros: string[] = [];
  let distrosLoading = false;
  let distrosError: string | null = null;
  let selectedDistro = '';

  let daemonStatus: string | null = null;
  let daemonError: string | null = null;
  let daemonLoading = false;

  const step = () => steps[stepIndex] ?? 'Welcome';

  const finishWizard = async () => {
    updateSettings({ firstRunComplete: true });
    await saveSettingsNow();
    await goto('/');
  };

  const skipWizard = async () => {
    await finishWizard();
  };

  const back = () => {
    stepIndex = Math.max(0, stepIndex - 1);
  };

  const next = async () => {
    if (step() === 'WSL') {
      updateSettings({ wslDistro: selectedDistro.trim() ? selectedDistro.trim() : null });
      await saveSettingsNow();
    }
    if (step() === 'Complete') {
      await finishWizard();
      return;
    }
    stepIndex = Math.min(steps.length - 1, stepIndex + 1);
  };

  const refreshDaemon = async () => {
    daemonLoading = true;
    daemonError = null;
    try {
      const response = await daemonHealth();
      daemonStatus = response.status;
      daemonError = response.lastError ?? null;
    } catch (caught) {
      daemonError =
        caught instanceof Error ? caught.message : caught ? String(caught) : 'Unable to check daemon';
    } finally {
      daemonLoading = false;
    }
  };

  const startDaemon = async () => {
    daemonLoading = true;
    daemonError = null;
    try {
      await daemonStart();
    } catch (caught) {
      daemonError =
        caught instanceof Error ? caught.message : caught ? String(caught) : 'Unable to start daemon';
    } finally {
      daemonLoading = false;
    }
    await refreshDaemon();
  };

  const testNotification = async () => {
    if (!('Notification' in window)) return;
    let permission = Notification.permission;
    if (permission !== 'granted') {
      permission = await Notification.requestPermission();
    }
    if (permission !== 'granted') return;
    new Notification('NTM Tracker', { body: 'Test notification from setup wizard.' });
  };

  onMount(() => {
    updateSettings({ firstRunComplete: false });

    void (async () => {
      distrosLoading = true;
      distrosError = null;
      try {
        distros = await listWslDistros();
      } catch (caught) {
        distrosError =
          caught instanceof Error ? caught.message : caught ? String(caught) : 'Unable to list distros';
        distros = [];
      } finally {
        distrosLoading = false;
      }

      const current = $settings.wslDistro;
      if (current) {
        selectedDistro = current;
      } else if (distros.length > 0) {
        selectedDistro = distros[0] ?? '';
      }
    })();

    void refreshDaemon();
  });
</script>

<main class="min-h-screen bg-slate-950 text-slate-100">
  <div class="mx-auto max-w-3xl px-6 py-12">
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div>
        <p class="text-sm uppercase tracking-[0.3em] text-slate-400">NTM Tracker</p>
        <h1 class="mt-2 text-3xl font-semibold text-white">First-run setup</h1>
        <p class="mt-2 text-sm text-slate-300/80">
          This wizard helps verify WSL connectivity, daemon health, and notifications.
        </p>
      </div>
      <button
        class="rounded-lg border border-slate-700 bg-slate-900 px-4 py-2 text-sm hover:bg-slate-800"
        on:click={skipWizard}
      >
        Skip
      </button>
    </div>

    <div class="mt-8 flex flex-wrap gap-2 text-xs">
      {#each steps as label, index (label)}
        <span
          class={`rounded-full px-3 py-1 ${
            index === stepIndex ? 'bg-sky-500/20 text-sky-200' : 'bg-slate-900 text-slate-300'
          }`}
          >{index + 1}. {label}</span
        >
      {/each}
    </div>

    <section class="mt-10 rounded-xl border border-slate-800 bg-slate-900/60 p-6">
      {#if step() === 'Welcome'}
        <h2 class="text-lg font-semibold text-white">Welcome</h2>
        <p class="mt-3 text-sm text-slate-300/80">
          NTM Tracker watches tmux/NTM sessions inside WSL2 and shows session status, compacts, and
          escalations in your Windows tray.
        </p>
        <p class="mt-3 text-sm text-slate-300/80">
          Privacy: the app only captures bounded pane output when you request a preview, and redacts
          known secrets server-side.
        </p>
      {:else if step() === 'WSL'}
        <h2 class="text-lg font-semibold text-white">WSL configuration</h2>
        <p class="mt-3 text-sm text-slate-300/80">
          Select the WSL distro that should host the daemon. If unsure, keep the default.
        </p>

        {#if distrosError}
          <div class="mt-4 rounded-lg border border-rose-500/40 bg-rose-500/10 px-4 py-3 text-sm text-rose-100">
            {distrosError}
          </div>
        {/if}

        <div class="mt-4 grid gap-2">
          <label class="grid gap-2 text-sm text-slate-200">
            WSL distro
            <select
              class="rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm disabled:opacity-50"
              bind:value={selectedDistro}
              disabled={distrosLoading || distros.length === 0}
            >
              {#if distrosLoading}
                <option>Loading...</option>
              {:else if distros.length === 0}
                <option value="">(No distros detected)</option>
              {:else}
                {#each distros as distro}
                  <option value={distro}>{distro}</option>
                {/each}
              {/if}
            </select>
          </label>
          <p class="text-xs text-slate-400">
            Current selection is stored in settings and used for WSL invocations.
          </p>
        </div>
      {:else if step() === 'Daemon'}
        <h2 class="text-lg font-semibold text-white">Daemon installation &amp; health</h2>
        <p class="mt-3 text-sm text-slate-300/80">
          The Windows app will bootstrap the daemon inside WSL. Use the buttons below to verify it
          starts successfully.
        </p>

        <div class="mt-4 flex flex-wrap gap-3">
          <button
            class="rounded-lg bg-indigo-500 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-400 disabled:opacity-50"
            on:click={startDaemon}
            disabled={daemonLoading}
          >
            Start daemon
          </button>
          <button
            class="rounded-lg border border-slate-700 bg-slate-900 px-4 py-2 text-sm hover:bg-slate-800 disabled:opacity-50"
            on:click={refreshDaemon}
            disabled={daemonLoading}
          >
            Refresh status
          </button>
        </div>

        <div class="mt-4 rounded-lg border border-slate-800 bg-slate-950/40 px-4 py-3 text-sm">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <span class="font-semibold text-white">Status: {daemonStatus ?? 'unknown'}</span>
            {#if daemonLoading}
              <span class="text-slate-400">Checking...</span>
            {/if}
          </div>
          {#if daemonError}
            <p class="mt-2 text-sm text-rose-200">{daemonError}</p>
          {/if}
        </div>
      {:else if step() === 'Notifications'}
        <h2 class="text-lg font-semibold text-white">Notifications</h2>
        <p class="mt-3 text-sm text-slate-300/80">
          Choose which events should show notifications. You can change these anytime in Settings.
        </p>

        <div class="mt-4 grid gap-4 sm:grid-cols-2">
          <label class="flex items-center gap-3">
            <input
              type="checkbox"
              checked={$settings.showNotifications}
              on:change={(event) =>
                updateSettings({
                  showNotifications: (event.target as HTMLInputElement).checked
                })}
            />
            Enable notifications
          </label>
          <label class="flex items-center gap-3">
            <input
              type="checkbox"
              checked={$settings.notifyOnCompact}
              on:change={(event) =>
                updateSettings({
                  notifyOnCompact: (event.target as HTMLInputElement).checked
                })}
            />
            Notify on compacts
          </label>
          <label class="flex items-center gap-3">
            <input
              type="checkbox"
              checked={$settings.notifyOnEscalation}
              on:change={(event) =>
                updateSettings({
                  notifyOnEscalation: (event.target as HTMLInputElement).checked
                })}
            />
            Notify on escalations
          </label>
          <div class="flex items-center gap-3">
            <button
              class="rounded-lg border border-slate-700 bg-slate-900 px-4 py-2 text-sm hover:bg-slate-800"
              on:click={testNotification}
            >
              Test notification
            </button>
          </div>
        </div>
      {:else if step() === 'Complete'}
        <h2 class="text-lg font-semibold text-white">Complete</h2>
        <p class="mt-3 text-sm text-slate-300/80">
          Setup is complete. You can re-run this wizard from Settings anytime.
        </p>
        <div class="mt-4 rounded-lg border border-slate-800 bg-slate-950/40 p-4 text-sm text-slate-300/80">
          <ul class="list-disc space-y-2 pl-6">
            <li>WSL distro: {$settings.wslDistro ?? '(default)'}</li>
            <li>Transport: {$settings.transport}</li>
            <li>Notifications: {$settings.showNotifications ? 'enabled' : 'disabled'}</li>
          </ul>
        </div>
      {/if}
    </section>

    <div class="mt-8 flex items-center justify-between">
      <button
        class="rounded-lg border border-slate-700 bg-slate-900 px-4 py-2 text-sm hover:bg-slate-800 disabled:opacity-50"
        on:click={back}
        disabled={stepIndex === 0}
      >
        Back
      </button>
      <button
        class="rounded-lg bg-indigo-500 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-400"
        on:click={next}
      >
        {#if step() === 'Complete'}Finish{:else}Next{/if}
      </button>
    </div>

    <div class="mt-10 text-center text-xs text-slate-500">
      <button class="underline hover:text-slate-300" on:click={() => goto('/')}>Return to dashboard</button>
    </div>
  </div>
</main>

