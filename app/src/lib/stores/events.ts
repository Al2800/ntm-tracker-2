import { writable } from 'svelte/store';
import type { TrackerEvent } from '../types';

const eventsStore = writable<TrackerEvent[]>([]);
const lastEventIdStore = writable<number>(0);

export const events = {
  subscribe: eventsStore.subscribe,
  set: eventsStore.set,
  update: eventsStore.update
};

export const lastEventId = {
  subscribe: lastEventIdStore.subscribe,
  set: lastEventIdStore.set,
  update: lastEventIdStore.update
};

export const appendEvents = (incoming: TrackerEvent[]) =>
  eventsStore.update((current) => {
    const merged = [...current, ...incoming];
    const maxId = merged.reduce((max, event) => Math.max(max, event.id), 0);
    lastEventIdStore.set(maxId);
    return merged;
  });

export const resetEvents = () => {
  eventsStore.set([]);
  lastEventIdStore.set(0);
};
