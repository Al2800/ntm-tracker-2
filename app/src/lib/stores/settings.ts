import { writable } from 'svelte/store';
import type { AppSettings } from '../types';

const defaultSettings: AppSettings = {
  transport: 'wsl-stdio',
  reconnectIntervalMs: 5000,
  showNotifications: true,
  notifyOnCompact: true,
  notifyOnEscalation: true,
  theme: 'system'
};

const settingsStore = writable<AppSettings>(defaultSettings);

export const settings = {
  subscribe: settingsStore.subscribe,
  set: settingsStore.set,
  update: settingsStore.update
};

export const resetSettings = () => settingsStore.set(defaultSettings);
export const updateSettings = (patch: Partial<AppSettings>) =>
  settingsStore.update((current) => ({ ...current, ...patch }));
