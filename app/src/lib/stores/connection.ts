import { writable } from 'svelte/store';
import type { ConnectionState } from '../types';

const connectionStateStore = writable<ConnectionState>('disconnected');
const lastHealthCheckStore = writable<Date | null>(null);

export const connectionState = {
  subscribe: connectionStateStore.subscribe,
  set: connectionStateStore.set,
  update: connectionStateStore.update
};

export const lastHealthCheck = {
  subscribe: lastHealthCheckStore.subscribe,
  set: lastHealthCheckStore.set,
  update: lastHealthCheckStore.update
};

export const setConnectionState = (state: ConnectionState) => connectionStateStore.set(state);
export const setLastHealthCheck = (timestamp: Date | null) => lastHealthCheckStore.set(timestamp);
