import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification
} from '@tauri-apps/plugin-notification';
import { get } from 'svelte/store';
import type { AppSettings, TrackerEvent } from '../types';
import { events } from '../stores/events';
import { settings } from '../stores/settings';
import { selectSession, sessions } from '../stores/sessions';

const DEDUPE_WINDOW_MS = 5 * 60 * 1000;
const MAX_DEDUPE_ENTRIES = 500;

let snoozedUntil: number | null = null;
let lastEventId: number | null = null;
let recentSent: number[] = [];
const lastByKey = new Map<string, number>();

/** Prune old entries from lastByKey to prevent unbounded memory growth */
const pruneLastByKey = () => {
  const cutoff = Date.now() - DEDUPE_WINDOW_MS;
  for (const [key, timestamp] of lastByKey) {
    if (timestamp < cutoff) {
      lastByKey.delete(key);
    }
  }
  // Additional safety: if still too large, remove oldest entries
  if (lastByKey.size > MAX_DEDUPE_ENTRIES) {
    const entries = [...lastByKey.entries()].sort((a, b) => a[1] - b[1]);
    const toRemove = entries.slice(0, lastByKey.size - MAX_DEDUPE_ENTRIES);
    for (const [key] of toRemove) {
      lastByKey.delete(key);
    }
  }
};

let currentSettings: AppSettings = get(settings);
let unsubscribeEvents: (() => void) | null = null;
let unsubscribeSettings: (() => void) | null = null;
let unlistenSnooze: UnlistenFn | null = null;

const isQuietHours = (now: Date) => {
  const hour = now.getHours();
  const quietStart = currentSettings.quietHoursStart;
  const quietEnd = currentSettings.quietHoursEnd;
  if (quietStart < quietEnd) {
    return hour >= quietStart && hour < quietEnd;
  }
  return hour >= quietStart || hour < quietEnd;
};

const isSnoozed = () => snoozedUntil !== null && Date.now() < snoozedUntil;

const pruneRecent = () => {
  const cutoff = Date.now() - 60 * 60 * 1000;
  recentSent = recentSent.filter((timestamp) => timestamp > cutoff);
};

export const snoozeForMinutes = (minutes: number) => {
  snoozedUntil = Date.now() + minutes * 60 * 1000;
};

export const snoozeUntilTomorrow = () => {
  const now = new Date();
  const target = new Date(now);
  target.setDate(now.getDate() + 1);
  target.setHours(currentSettings.quietHoursEnd, 0, 0, 0);
  snoozedUntil = target.getTime();
};

const ensurePermission = async () => {
  if (!currentSettings.showNotifications) return false;
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
    }
    return granted;
  } catch {
    // Fallback: check browser API if Tauri plugin unavailable
    if ('Notification' in window) {
      return Notification.permission === 'granted';
    }
    return false;
  }
};

const shouldNotify = (event: TrackerEvent) => {
  if (!currentSettings.showNotifications) return false;
  if (event.type === 'compact' && !currentSettings.notifyOnCompact) return false;
  if (event.type === 'escalation' && !currentSettings.notifyOnEscalation) return false;
  if (isQuietHours(new Date())) return false;
  if (isSnoozed()) return false;

  pruneRecent();
  pruneLastByKey();
  if (recentSent.length >= currentSettings.notificationMaxPerHour) return false;

  const key = `${event.type}:${event.sessionUid}`;
  const lastSeen = lastByKey.get(key);
  if (lastSeen && Date.now() - lastSeen < DEDUPE_WINDOW_MS) {
    return false;
  }
  lastByKey.set(key, Date.now());
  recentSent.push(Date.now());
  return true;
};

const sessionLabel = (event: TrackerEvent) => {
  const sessionList = get(sessions);
  const session = sessionList.find((item) => item.sessionUid === event.sessionUid);
  return session?.name ?? event.sessionUid;
};

const notificationBody = (event: TrackerEvent) => {
  const sessionName = sessionLabel(event);
  const paneLabel = event.paneUid;
  if (event.type === 'compact') {
    const tokenMatch = event.message?.match(/\d+/)?.[0];
    const tokenInfo = tokenMatch ? ` (was ~${tokenMatch} tokens)` : '';
    return `${sessionName}:${paneLabel} auto-compacted${tokenInfo}`;
  }
  const summary = event.message ? ` — ${event.message.slice(0, 100)}` : '';
  return `${sessionName}:${paneLabel} needs attention${summary}`;
};

const focusEvent = (event: TrackerEvent) => {
  selectSession(event.sessionUid);
  window.focus();
};

const notifyEvent = async (event: TrackerEvent) => {
  if (!shouldNotify(event)) return;
  const granted = await ensurePermission();
  if (!granted) return;

  const title = event.type === 'compact' ? 'Context Compacted' : 'Escalation';
  const body = notificationBody(event);

  // Use Tauri's notification plugin for proper native notifications
  try {
    sendNotification({ title, body });
  } catch {
    // Fallback to browser API if Tauri plugin fails (e.g., in dev mode)
    if ('Notification' in window) {
      const notification = new Notification(title, { body });
      notification.onclick = () => focusEvent(event);
    }
  }
};

const handleEventsUpdate = (current: TrackerEvent[]) => {
  if (!Array.isArray(current) || current.length === 0) return;
  const maxId = current.reduce((max, event) => Math.max(max, event.id), 0);
  if (lastEventId === null) {
    lastEventId = maxId;
    return;
  }
  const newEvents = current.filter((event) => event.id > lastEventId).sort((a, b) => a.id - b.id);
  lastEventId = maxId;
  newEvents.forEach((event) => {
    if (event.type === 'compact' || event.type === 'escalation') {
      void notifyEvent(event);
    }
  });
};

export const initNotifications = () => {
  if (unsubscribeEvents) return;
  unsubscribeSettings = settings.subscribe((next) => {
    currentSettings = next;
  });
  void ensurePermission();
  void listen('tray:snooze', () => snoozeForMinutes(15)).then((unlisten) => {
    unlistenSnooze = unlisten;
  });
  unsubscribeEvents = events.subscribe(handleEventsUpdate);
};

export const stopNotifications = () => {
  unsubscribeEvents?.();
  unsubscribeEvents = null;
  unsubscribeSettings?.();
  unsubscribeSettings = null;
  if (unlistenSnooze) {
    void unlistenSnooze();
    unlistenSnooze = null;
  }
};
