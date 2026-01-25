import { get, writable } from 'svelte/store';
import type { AppSettings } from '../types';
import { getSettings, setSettings } from '../tauri';

const defaultSettings: AppSettings = {
  transport: 'wsl-stdio',
  reconnectIntervalMs: 5000,
  autostartEnabled: true,
  showNotifications: true,
  notifyOnCompact: true,
  notifyOnEscalation: true,
  theme: 'system'
};

const settingsStore = writable<AppSettings>(defaultSettings);

let initialized = false;
let hydrating = false;
let persistTimer: ReturnType<typeof setTimeout> | null = null;

export const settings = {
  subscribe: settingsStore.subscribe,
  set: settingsStore.set,
  update: settingsStore.update
};

export const resetSettings = () => settingsStore.set(defaultSettings);
export const updateSettings = (patch: Partial<AppSettings>) =>
  settingsStore.update((current) => ({ ...current, ...patch }));

export const saveSettingsNow = async () => {
  const current = get(settingsStore);
  await setSettings(current);
};

export const initSettings = () => {
  if (initialized) return;
  initialized = true;

  void (async () => {
    try {
      const remote = await getSettings();
      hydrating = true;
      settingsStore.set(remote);
    } catch {
      // Keep defaults if unavailable.
    } finally {
      hydrating = false;
    }
  })();

  settingsStore.subscribe((value) => {
    if (hydrating) return;
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      void setSettings(value);
    }, 300);
  });
};
