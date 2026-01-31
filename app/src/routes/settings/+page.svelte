<svelte:head>
  <title>Settings · NTM Tracker</title>
</svelte:head>

<script lang="ts">
  import { goto } from '$app/navigation';
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

  const runWizard = async () => {
    updateSettings({ firstRunComplete: false });
    await saveSettingsNow();
    await goto('/wizard');
  };
</script>

<main class="min-h-screen bg-surface-base text-text-primary">
  <div class="mx-auto max-w-4xl px-6 py-12">
    <div class="flex flex-wrap items-end justify-between gap-4">
      <div>
        <p class="label">NTM Tracker</p>
        <h1 class="mt-2 text-3xl font-semibold text-text-primary">Settings</h1>
      </div>
      <div class="flex items-center gap-3">
        <button class="btn btn-secondary" on:click={onReset} disabled={saving}>
          Reset
        </button>
        <button class="btn btn-primary" on:click={onSave} disabled={saving}>
          {#if saving}Saving...{:else}Save{/if}
        </button>
      </div>
    </div>

    {#if saveError}
      <div class="mt-6 card card-critical text-sm text-status-error-text">
        {saveError}
      </div>
    {/if}

    <div class="mt-10 space-y-8">
      <section class="card">
        <h2 class="text-lg font-semibold text-text-primary">Connection</h2>
        <div class="mt-4 grid gap-4 sm:grid-cols-2">
          <label class="grid gap-2 text-sm text-text-secondary">
            <span class="label-sm">Transport</span>
            <select
              class="input"
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

          <label class="grid gap-2 text-sm text-text-secondary">
            <span class="label-sm">Reconnect interval (ms)</span>
            <input
              class="input"
              type="number"
              min="1000"
              step="250"
              value={$settings.reconnectIntervalMs}
              on:input={onReconnectInterval}
            />
          </label>

          <label class="flex items-center gap-3 text-sm text-text-secondary">
            <input
              type="checkbox"
              checked={$settings.autostartEnabled}
              on:change={(event) =>
                updateSettings({ autostartEnabled: parseBool((event.target as HTMLInputElement).checked) })}
            />
            Launch on startup
          </label>

          <label class="grid gap-2 text-sm text-text-secondary">
            <span class="label-sm">WSL distro (optional)</span>
            <input
              class="input"
              placeholder="default"
              value={$settings.wslDistro ?? ''}
              on:input={(event) => {
                const value = (event.target as HTMLInputElement).value.trim();
                updateSettings({ wslDistro: value.length === 0 ? null : value });
              }}
            />
          </label>
        </div>
      </section>

      <section class="card">
        <div class="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 class="text-lg font-semibold text-text-primary">Onboarding</h2>
            <p class="mt-1 text-sm text-text-secondary">
              Re-run the first-run wizard to verify WSL + notifications.
            </p>
          </div>
          <button class="btn btn-secondary" on:click={runWizard}>
            Run wizard
          </button>
        </div>
      </section>

      <section class="card">
        <h2 class="text-lg font-semibold text-text-primary">Notifications</h2>
        <div class="mt-4 grid gap-4 text-sm text-text-secondary sm:grid-cols-2">
          <label class="flex items-center gap-3">
            <input
              type="checkbox"
              checked={$settings.showNotifications}
              on:change={(event) =>
                updateSettings({ showNotifications: parseBool((event.target as HTMLInputElement).checked) })}
            />
            Enable notifications
          </label>
          <label class="flex items-center gap-3">
            <input
              type="checkbox"
              checked={$settings.notifyOnCompact}
              on:change={(event) =>
                updateSettings({ notifyOnCompact: parseBool((event.target as HTMLInputElement).checked) })}
            />
            Notify on compact events
          </label>
          <label class="flex items-center gap-3">
            <input
              type="checkbox"
              checked={$settings.notifyOnEscalation}
              on:change={(event) =>
                updateSettings({ notifyOnEscalation: parseBool((event.target as HTMLInputElement).checked) })}
            />
            Notify on escalations
          </label>

          <label class="grid gap-2">
            <span class="label-sm">Quiet hours start (0-23)</span>
            <input
              class="input"
              type="number"
              min="0"
              max="23"
              step="1"
              value={$settings.quietHoursStart}
              on:input={onHourField('quietHoursStart')}
            />
          </label>
          <label class="grid gap-2">
            <span class="label-sm">Quiet hours end (0-23)</span>
            <input
              class="input"
              type="number"
              min="0"
              max="23"
              step="1"
              value={$settings.quietHoursEnd}
              on:input={onHourField('quietHoursEnd')}
            />
          </label>
          <label class="grid gap-2">
            <span class="label-sm">Max notifications / hour</span>
            <input
              class="input"
              type="number"
              min="1"
              step="1"
              value={$settings.notificationMaxPerHour}
              on:input={onMaxPerHour}
            />
          </label>
        </div>
      </section>

      <section class="card">
        <h2 class="text-lg font-semibold text-text-primary">Appearance</h2>
        <div class="mt-4 grid gap-4 text-sm text-text-secondary sm:grid-cols-2">
          <label class="grid gap-2">
            <span class="label-sm">Theme</span>
            <select
              class="input"
              value={$settings.theme}
              on:change={(event) =>
                updateSettings({ theme: (event.target as HTMLSelectElement).value as AppSettings['theme'] })}
            >
              {#each themeOptions as option}
                <option value={option}>{option}</option>
              {/each}
            </select>
          </label>
          <label class="grid gap-2">
            <span class="label-sm">Log level</span>
            <select
              class="input"
              value={$settings.logLevel}
              on:change={(event) =>
                updateSettings({ logLevel: (event.target as HTMLSelectElement).value as AppSettings['logLevel'] })}
            >
              {#each logLevelOptions as option}
                <option value={option}>{option}</option>
              {/each}
            </select>
          </label>
        </div>
      </section>
    </div>
  </div>
</main>
