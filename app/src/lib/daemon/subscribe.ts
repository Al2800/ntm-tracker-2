import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import { rpcCallWithRetry } from '../tauri';
import type { DailyStats, HourlyStats, Session, TrackerEvent } from '../types';
import { appendEvents, lastEventId, resetEvents } from '../stores/events';
import { setSessions, upsertSession } from '../stores/sessions';
import { setDailyStats, setHourlyStats } from '../stores/stats';

type DaemonEventPayload = {
  method: string;
  params?: Record<string, unknown>;
};

let unlisten: UnlistenFn | null = null;
let subscriptionActive = false;

const isStaleCursorError = (error: unknown) =>
  typeof error === 'string' && error.toUpperCase().includes('STALE_CURSOR');

const applySnapshot = (snapshot: Record<string, unknown>) => {
  if (Array.isArray(snapshot.sessions)) {
    setSessions(snapshot.sessions as Session[]);
  }
  if (Array.isArray(snapshot.events)) {
    resetEvents();
    appendEvents(snapshot.events as TrackerEvent[]);
  }
  const stats = snapshot.stats as Record<string, unknown> | undefined;
  if (stats && Array.isArray(stats.hourly)) {
    setHourlyStats(stats.hourly as HourlyStats[]);
  }
  if (stats && Array.isArray(stats.daily)) {
    setDailyStats(stats.daily as DailyStats[]);
  }
};

const handleDaemonEvent = (payload: DaemonEventPayload) => {
  const { method, params } = payload;
  switch (method) {
    case 'session.delta':
      if (params?.session) {
        upsertSession(params.session as Session);
      }
      break;
    case 'sessions.snapshot':
      if (params) applySnapshot(params);
      break;
    case 'events':
      if (Array.isArray(params?.events)) {
        appendEvents(params.events as TrackerEvent[]);
      }
      break;
    case 'stats.hourly':
      if (Array.isArray(params?.hourly)) {
        setHourlyStats(params.hourly as HourlyStats[]);
      }
      break;
    case 'stats.daily':
      if (Array.isArray(params?.daily)) {
        setDailyStats(params.daily as DailyStats[]);
      }
      break;
    default:
      break;
  }
};

export const startDaemonSubscription = async (channels = ['sessions', 'events', 'stats']) => {
  if (subscriptionActive) return;
  subscriptionActive = true;
  const sinceEventId = get(lastEventId);
  let snapshotFetched = false;

  try {
    await rpcCallWithRetry('subscribe', { channels, sinceEventId });
  } catch (error) {
    if (isStaleCursorError(error)) {
      resetEvents();
      await rpcCallWithRetry('subscribe', { channels, sinceEventId: 0 });
      const snapshot = await rpcCallWithRetry<Record<string, unknown>>('snapshot.get');
      applySnapshot(snapshot);
      snapshotFetched = true;
    } else {
      subscriptionActive = false;
      throw error;
    }
  }

  if (!snapshotFetched && sinceEventId === 0) {
    const snapshot = await rpcCallWithRetry<Record<string, unknown>>('snapshot.get');
    applySnapshot(snapshot);
  }

  if (!unlisten) {
    unlisten = await listen<DaemonEventPayload>('daemon-event', (event) => {
      handleDaemonEvent(event.payload);
    });
  }
};

export const stopDaemonSubscription = async () => {
  if (unlisten) {
    await unlisten();
    unlisten = null;
  }
  subscriptionActive = false;
};
