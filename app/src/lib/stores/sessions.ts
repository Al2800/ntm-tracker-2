import { derived, writable } from 'svelte/store';
import type { Session } from '../types';

const sessionsStore = writable<Session[]>([]);
const selectedSessionIdStore = writable<string | null>(null);
const pinnedSessionIdsStore = writable<Set<string>>(new Set());

// Load pinned sessions from localStorage
if (typeof window !== 'undefined') {
  try {
    const stored = localStorage.getItem('ntm-pinned-sessions');
    if (stored) {
      const parsed = JSON.parse(stored);
      // Validate that parsed value is an array of strings
      if (Array.isArray(parsed) && parsed.every((item) => typeof item === 'string')) {
        pinnedSessionIdsStore.set(new Set(parsed));
      }
    }
  } catch {
    // Ignore localStorage errors - will use empty Set default
  }
}

export const sessions = {
  subscribe: sessionsStore.subscribe,
  set: sessionsStore.set,
  update: sessionsStore.update
};

export const selectedSessionId = {
  subscribe: selectedSessionIdStore.subscribe,
  set: selectedSessionIdStore.set,
  update: selectedSessionIdStore.update
};

export const pinnedSessionIds = {
  subscribe: pinnedSessionIdsStore.subscribe
};

export const selectedSession = derived(
  [sessionsStore, selectedSessionIdStore],
  ([$sessions, $selectedId]) => $sessions.find((session) => session.sessionUid === $selectedId) ?? null
);

export const setSessions = (next: Session[]) => sessionsStore.set(next);

export const upsertSession = (session: Session) =>
  sessionsStore.update((current) => {
    const index = current.findIndex((item) => item.sessionUid === session.sessionUid);
    if (index === -1) {
      return [...current, session];
    }
    const updated = [...current];
    updated[index] = { ...updated[index], ...session };
    return updated;
  });

export const selectSession = (sessionUid: string | null) => selectedSessionIdStore.set(sessionUid);

export const togglePinSession = (sessionUid: string) => {
  pinnedSessionIdsStore.update((current) => {
    const next = new Set(current);
    if (next.has(sessionUid)) {
      next.delete(sessionUid);
    } else {
      next.add(sessionUid);
    }
    // Persist to localStorage
    if (typeof window !== 'undefined') {
      try {
        localStorage.setItem('ntm-pinned-sessions', JSON.stringify([...next]));
      } catch {
        // Ignore localStorage errors
      }
    }
    return next;
  });
};
