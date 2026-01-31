import { derived, writable } from 'svelte/store';
import type { Pane, Session } from '../types';

const sessionsStore = writable<Session[]>([]);
const selectedSessionIdStore = writable<string | null>(null);
const pinnedSessionIdsStore = writable<Set<string>>(new Set());
const mutedSessionIdsStore = writable<Set<string>>(new Set());

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

// Load muted sessions from localStorage
if (typeof window !== 'undefined') {
  try {
    const stored = localStorage.getItem('ntm-muted-sessions');
    if (stored) {
      const parsed = JSON.parse(stored);
      if (Array.isArray(parsed) && parsed.every((item) => typeof item === 'string')) {
        mutedSessionIdsStore.set(new Set(parsed));
      }
    }
  } catch {
    // Ignore localStorage errors
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

export const mutedSessionIds = {
  subscribe: mutedSessionIdsStore.subscribe
};

export const selectedSession = derived(
  [sessionsStore, selectedSessionIdStore],
  ([$sessions, $selectedId]) => $sessions.find((session) => session.sessionId === $selectedId) ?? null
);

const panesEqual = (a: Pane[] | undefined, b: Pane[] | undefined) => {
  const left = a ?? [];
  const right = b ?? [];
  if (left === right) return true;
  if (left.length !== right.length) return false;
  const rightById = new Map(right.map((pane) => [pane.paneId, pane]));
  return left.every((pane) => {
    const other = rightById.get(pane.paneId);
    if (!other) return false;
    return (
      pane.sessionId === other.sessionId &&
      pane.paneIndex === other.paneIndex &&
      pane.status === other.status &&
      pane.agentType === other.agentType &&
      pane.lastActivityAt === other.lastActivityAt &&
      pane.currentCommand === other.currentCommand &&
      pane.tmuxPaneId === other.tmuxPaneId &&
      pane.tmuxWindowId === other.tmuxWindowId &&
      pane.tmuxPanePid === other.tmuxPanePid
    );
  });
};

const sessionsEqual = (current: Session[], next: Session[]) => {
  if (current === next) return true;
  if (current.length !== next.length) return false;
  const currentById = new Map(current.map((session) => [session.sessionId, session]));
  return next.every((session) => {
    const existing = currentById.get(session.sessionId);
    if (!existing) return false;
    return (
      existing.name === session.name &&
      existing.status === session.status &&
      existing.paneCount === session.paneCount &&
      existing.lastSeenAt === session.lastSeenAt &&
      existing.tmuxSessionId === session.tmuxSessionId &&
      existing.sourceId === session.sourceId &&
      panesEqual(existing.panes, session.panes)
    );
  });
};

export const setSessions = (next: Session[]) =>
  sessionsStore.update((current) => (sessionsEqual(current, next) ? current : next));

export const upsertSession = (session: Session) =>
  sessionsStore.update((current) => {
    const index = current.findIndex((item) => item.sessionId === session.sessionId);
    if (index === -1) {
      return [...current, session];
    }
    const updated = [...current];
    updated[index] = { ...updated[index], ...session };
    return updated;
  });

export const selectSession = (sessionId: string | null) => selectedSessionIdStore.set(sessionId);

export const togglePinSession = (sessionId: string) => {
  pinnedSessionIdsStore.update((current) => {
    const next = new Set(current);
    if (next.has(sessionId)) {
      next.delete(sessionId);
    } else {
      next.add(sessionId);
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

export const toggleMuteSession = (sessionId: string) => {
  mutedSessionIdsStore.update((current) => {
    const next = new Set(current);
    if (next.has(sessionId)) {
      next.delete(sessionId);
    } else {
      next.add(sessionId);
    }
    if (typeof window !== 'undefined') {
      try {
        localStorage.setItem('ntm-muted-sessions', JSON.stringify([...next]));
      } catch {
        // Ignore localStorage errors
      }
    }
    return next;
  });
};
