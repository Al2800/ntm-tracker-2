import { writable } from 'svelte/store';
import type { ConnectionState } from '../types';
import { daemonHealth, daemonStart } from '../tauri';
import { settings } from './settings';

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

let connectionLoopRunning = false;
let reconnectAttempt = 0;
let timeoutHandle: ReturnType<typeof setTimeout> | null = null;

const scheduleNext = (ms: number, fn: () => void) => {
  if (timeoutHandle) clearTimeout(timeoutHandle);
  timeoutHandle = setTimeout(fn, ms);
};

const backoffMs = (base: number) => {
  const cappedAttempt = Math.min(reconnectAttempt, 5);
  return Math.min(30000, base * 2 ** cappedAttempt);
};

export const startConnectionLoop = () => {
  if (connectionLoopRunning) return;
  connectionLoopRunning = true;
  reconnectAttempt = 0;

  const tick = async () => {
    if (!connectionLoopRunning) return;
    connectionStateStore.update((state) => (state === 'disconnected' ? 'connecting' : state));

    let intervalMs = 5000;
    settings.subscribe((current) => {
      intervalMs = current.reconnectIntervalMs;
    })();

    try {
      const health = await daemonHealth();
      lastHealthCheckStore.set(new Date());

      if (health.status === 'running') {
        reconnectAttempt = 0;
        connectionStateStore.set('connected');
        scheduleNext(intervalMs, tick);
        return;
      }

      connectionStateStore.set('reconnecting');
      await daemonStart();
      reconnectAttempt += 1;
      scheduleNext(backoffMs(intervalMs), tick);
    } catch {
      connectionStateStore.set('reconnecting');
      reconnectAttempt += 1;
      scheduleNext(backoffMs(intervalMs), tick);
    }
  };

  void tick();
};

export const stopConnectionLoop = () => {
  connectionLoopRunning = false;
  reconnectAttempt = 0;
  if (timeoutHandle) clearTimeout(timeoutHandle);
  timeoutHandle = null;
  connectionStateStore.set('disconnected');
  lastHealthCheckStore.set(null);
};
