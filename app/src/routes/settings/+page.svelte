<svelte:head>
  <title>Settings · NTM Tracker</title>
</svelte:head>

<script lang="ts">
  import { settings, resetSettings, saveSettingsNow, updateSettings } from '$lib/stores/settings';
  import type { AppSettings } from '$lib/types';

  let saving = false;
  let saveError: string | null = null;

  const transportOptions: AppSettings['transport'][] = ['wsl-stdio', 'ws', 'http'];
  const themeOptions: AppSettings['theme'][] = ['system', 'light', 'dark'];
  const logLevelOptions: AppSettings['logLevel'][] = ['trace', 'debug', 'info', 'warn', 'error'];

  const parseBool = (value: unknown) => Boolean(value);

  const onReconnectInterval = (event: Event) => {
    const value = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (!Number.isFinite(value) || value <= 0) return;
    updateSettings({ reconnectIntervalMs: value });
  };

  const onSave = async () => {
    saving = true;
    saveError = null;
    try {
      await saveSettingsNow();
    } catch (error) {
      saveError =
        error instanceof Error ? error.message : error ? String(error) : 'Unable to save settings';
    } finally {
      saving = false;
    }
  };

  const onReset = async () => {
    resetSettings();
    await onSave();
  };

  const onHourField = (field: 'quietHoursStart' | 'quietHoursEnd') => (event: Event) => {
    const value = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (!Number.isFinite(value) || value < 0 || value > 23) return;
    updateSettings({ [field]: value } as Partial<AppSettings>);
  };

  const onMaxPerHour = (event: Event) => {
    const value = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (!Number.isFinite(value) || value <= 0) return;
    updateSettings({ notificationMaxPerHour: value });
  };
</script>

<main class="min-h-screen bg-slate-950 text-slate-100">
  <div class="mx-auto max-w-4xl px-6 py-12">
    <div class="flex flex-wrap items-end justify-between gap-4">
      <div>
        <p class="text-sm uppercase tracking-[0.3em] text-slate-400">NTM Tracker</p>
        <h1 class="mt-2 text-3xl font-semibold text-white">Settings</h1>
      </div>
      <div class="flex items-center gap-3">
        <button
          class="rounded-lg border border-slate-700 bg-slate-900 px-4 py-2 text-sm hover:bg-slate-800"
          on:click={onReset}
          disabled={saving}
        >
          Reset
        </button>
        <button
          class="rounded-lg bg-indigo-500 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-400 disabled:opacity-50"
          on:click={onSave}
          disabled={saving}
        >
          {#if saving}Saving...{:else}Save{/if}
        </button>
      </div>
    </div>

    {#if saveError}
      <div class="mt-6 rounded-lg border border-rose-500/40 bg-rose-500/10 px-4 py-3 text-sm text-rose-100">
        {saveError}
      </div>
    {/if}

    <div class="mt-10 space-y-8">
      <section class="rounded-xl border border-slate-800 bg-slate-900/60 p-6">
        <h2 class="text-lg font-semibold text-white">Connection</h2>
        <div class="mt-4 grid gap-4 sm:grid-cols-2">
          <label class="grid gap-2 text-sm text-slate-200">
            Transport
            <select
              class="rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
              value={$settings.transport}
              on:change={(event) =>
                updateSettings({
                  transport: (event.target as HTMLSelectElement).value as AppSettings['transport']
                })}
            >
              {#each transportOptions as option}
                <option value={option}>{option}</option>
              {/each}
            </select>
          </label>

          <label class="grid gap-2 text-sm text-slate-200">
            Reconnect interval (ms)
            <input
              class="rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
              type="number"
              min="1000"
              step="250"
              value={$settings.reconnectIntervalMs}
              on:input={onReconnectInterval}
            />
          </label>

          <label class="flex items-center gap-3 text-sm text-slate-200">
            <input
              type="checkbox"
              checked={$settings.autostartEnabled}
              on:change={(event) => updateSettings({ autostartEnabled: parseBool((event.target as HTMLInputElement).checked) })}
            />
            Launch on startup
          </label>

          <div class="text-sm text-slate-300/80">
            <p class="font-semibold text-slate-200">WSL distro</p>
            <p class="mt-1">
              Automatic selection uses the default WSL distro. Multi-distro selection
              is not implemented yet.
            </p>
          </div>
        </div>
      </section>

      <section class="rounded-xl border border-slate-800 bg-slate-900/60 p-6">
        <h2 class="text-lg font-semibold text-white">Notifications</h2>
        <div class="mt-4 grid gap-4 text-sm text-slate-200 sm:grid-cols-2">
          <label class="flex items-center gap-3">
            <input
              type="checkbox"
              checked={$settings.showNotifications}
              on:change={(event) => updateSettings({ showNotifications: parseBool((event.target as HTMLInputElement).checked) })}
            />
            Enable notifications
          </label>
          <label class="flex items-center gap-3">
            <input
              type="checkbox"
              checked={$settings.notifyOnCompact}
              on:change={(event) => updateSettings({ notifyOnCompact: parseBool((event.target as HTMLInputElement).checked) })}
            />
            Notify on compact events
          </label>
          <label class="flex items-center gap-3">
            <input
              type="checkbox"
              checked={$settings.notifyOnEscalation}
              on:change={(event) => updateSettings({ notifyOnEscalation: parseBool((event.target as HTMLInputElement).checked) })}
            />
            Notify on escalations
          </label>

          <label class="grid gap-2">
            Quiet hours start (0-23)
            <input
              class="rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
              type="number"
              min="0"
              max="23"
              step="1"
              value={$settings.quietHoursStart}
              on:input={onHourField('quietHoursStart')}
            />
          </label>

          <label class="grid gap-2">
            Quiet hours end (0-23)
            <input
              class="rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
              type="number"
              min="0"
              max="23"
              step="1"
              value={$settings.quietHoursEnd}
              on:input={onHourField('quietHoursEnd')}
            />
          </label>

          <label class="grid gap-2">
            Max notifications per hour
            <input
              class="rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
              type="number"
              min="1"
              step="1"
              value={$settings.notificationMaxPerHour}
              on:input={onMaxPerHour}
            />
          </label>
        </div>
      </section>

      <section class="rounded-xl border border-slate-800 bg-slate-900/60 p-6">
        <h2 class="text-lg font-semibold text-white">Appearance</h2>
        <div class="mt-4 grid gap-4 sm:grid-cols-2">
          <label class="grid gap-2 text-sm text-slate-200">
            Theme
            <select
              class="rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
              value={$settings.theme}
              on:change={(event) =>
                updateSettings({
                  theme: (event.target as HTMLSelectElement).value as AppSettings['theme']
                })}
            >
              {#each themeOptions as option}
                <option value={option}>{option}</option>
              {/each}
            </select>
          </label>
        </div>
      </section>

      <section class="rounded-xl border border-slate-800 bg-slate-900/60 p-6">
        <h2 class="text-lg font-semibold text-white">Advanced</h2>
        <div class="mt-4 grid gap-4 sm:grid-cols-2">
          <label class="flex items-center gap-3 text-sm text-slate-200">
            <input
              type="checkbox"
              checked={$settings.debugMode}
              on:change={(event) => updateSettings({ debugMode: parseBool((event.target as HTMLInputElement).checked) })}
            />
            Debug mode
          </label>

          <label class="grid gap-2 text-sm text-slate-200">
            Log level
            <select
              class="rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
              value={$settings.logLevel}
              on:change={(event) =>
                updateSettings({
                  logLevel: (event.target as HTMLSelectElement).value as AppSettings['logLevel']
                })}
            >
              {#each logLevelOptions as option}
                <option value={option}>{option}</option>
              {/each}
            </select>
          </label>

          <div class="text-sm text-slate-300/80 sm:col-span-2">
            Poll intervals and daemon-side debug flags are not configurable yet.
          </div>
        </div>
      </section>

      <section class="rounded-xl border border-slate-800 bg-slate-900/60 p-6">
        <h2 class="text-lg font-semibold text-white">About</h2>
        <div class="mt-3 text-sm text-slate-300/80">
          Settings are stored in the app config directory and applied immediately.
          Upgrade checks and detailed version reporting will be added next.
        </div>
      </section>
    </div>
  </div>
</main>
