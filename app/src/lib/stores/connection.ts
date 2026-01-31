import { get, writable } from 'svelte/store';
import type { ConnectionState } from '../types';
import { daemonHealth, daemonStart } from '../tauri';
import { startDaemonSubscription, stopDaemonSubscription } from '../daemon/subscribe';
import { settings } from './settings';

const connectionStateStore = writable<ConnectionState>('disconnected');
const lastHealthCheckStore = writable<Date | null>(null);
const lastErrorStore = writable<string | null>(null);

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

export const lastConnectionError = {
  subscribe: lastErrorStore.subscribe,
  set: lastErrorStore.set,
  update: lastErrorStore.update
};

export const setConnectionState = (state: ConnectionState) => connectionStateStore.set(state);
export const setLastHealthCheck = (timestamp: Date | null) => lastHealthCheckStore.set(timestamp);
export const setLastConnectionError = (message: string | null) => lastErrorStore.set(message);

let connectionLoopRunning = false;
let reconnectAttempt = 0;
let timeoutHandle: ReturnType<typeof setTimeout> | null = null;
let subscriptionActive = false;

const scheduleNext = (ms: number, fn: () => void) => {
  if (timeoutHandle) clearTimeout(timeoutHandle);
  timeoutHandle = setTimeout(fn, ms);
};

const backoffMs = (base: number) => {
  const cappedAttempt = Math.min(reconnectAttempt, 5);
  return Math.min(30000, base * 2 ** cappedAttempt);
};

const isVersionMismatch = (message: string) =>
  /incompatible|protocolversion=|schemaversion=/i.test(message);

export const startConnectionLoop = () => {
  if (connectionLoopRunning) return;
  connectionLoopRunning = true;
  reconnectAttempt = 0;

  const tick = async () => {
    if (!connectionLoopRunning) return;
    connectionStateStore.update((state) => (state === 'disconnected' ? 'connecting' : state));

    // Use get() to read synchronously - no subscription leak
    const intervalMs = get(settings).reconnectIntervalMs;

    try {
      const health = await daemonHealth();
      lastHealthCheckStore.set(new Date());

      if (health.status === 'running') {
        reconnectAttempt = 0;
        connectionStateStore.set('connected');
        lastErrorStore.set(null);
        if (!subscriptionActive) {
          try {
            await startDaemonSubscription();
            subscriptionActive = true;
          } catch (error) {
            const message =
              error instanceof Error
                ? error.message
                : error
                  ? String(error)
                  : 'Unable to start daemon subscription';
            lastErrorStore.set(message);
            subscriptionActive = false;
          }
        }
        scheduleNext(intervalMs, tick);
        return;
      }

      if (subscriptionActive) {
        await stopDaemonSubscription();
        subscriptionActive = false;
      }

      if (health.lastError) {
        lastErrorStore.set(health.lastError);
      } else {
        lastErrorStore.set('Daemon is not running');
      }

      const message = health.lastError ?? 'Daemon is not running';
      if (isVersionMismatch(message)) {
        connectionStateStore.set('degraded');
        scheduleNext(intervalMs, tick);
        return;
      }

      connectionStateStore.set('reconnecting');
      if (subscriptionActive) {
        await stopDaemonSubscription();
        subscriptionActive = false;
      }
      await daemonStart();
      reconnectAttempt += 1;
      scheduleNext(backoffMs(intervalMs), tick);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : error ? String(error) : 'Unable to reach daemon';
      lastErrorStore.set(message);

      if (subscriptionActive) {
        await stopDaemonSubscription();
        subscriptionActive = false;
      }

      if (isVersionMismatch(message)) {
        connectionStateStore.set('degraded');
        scheduleNext(intervalMs, tick);
        return;
      }

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
  if (subscriptionActive) {
    void stopDaemonSubscription();
    subscriptionActive = false;
  }
  connectionStateStore.set('disconnected');
  lastHealthCheckStore.set(null);
  lastErrorStore.set(null);
};
