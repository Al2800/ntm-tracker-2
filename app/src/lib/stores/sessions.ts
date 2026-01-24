import { derived, writable } from 'svelte/store';
import type { Session } from '../types';

const sessionsStore = writable<Session[]>([]);
const selectedSessionIdStore = writable<string | null>(null);

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
