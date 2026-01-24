import { writable } from 'svelte/store';
import type { DailyStats, HourlyStats } from '../types';

const hourlyStore = writable<HourlyStats[]>([]);
const dailyStore = writable<DailyStats[]>([]);

export const hourlyStats = {
  subscribe: hourlyStore.subscribe,
  set: hourlyStore.set,
  update: hourlyStore.update
};

export const dailyStats = {
  subscribe: dailyStore.subscribe,
  set: dailyStore.set,
  update: dailyStore.update
};

export const setHourlyStats = (stats: HourlyStats[]) => hourlyStore.set(stats);
export const setDailyStats = (stats: DailyStats[]) => dailyStore.set(stats);
