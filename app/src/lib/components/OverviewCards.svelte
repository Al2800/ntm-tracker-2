<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { sessions } from '../stores/sessions';
  import { events } from '../stores/events';
  import { dailyStats } from '../stores/stats';
  import { connectionState } from '../stores/connection';
  import { getConnectionStatus } from '../status';

  const normalizeTimestamp = (value: number) => (value < 1_000_000_000_000 ? value * 1000 : value);

  const getTodayStartMs = () => {
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    return now.getTime();
  };

  // Recalculate at midnight or when stores update
  let todayStartMs = getTodayStartMs();
  let midnightTimer: ReturnType<typeof setTimeout> | null = null;

  const scheduleMidnightRefresh = () => {
    const now = Date.now();
    const msUntilMidnight = getTodayStartMs() + 24 * 60 * 60 * 1000 - now;
    midnightTimer = setTimeout(() => {
      todayStartMs = getTodayStartMs();
      scheduleMidnightRefresh();
    }, msUntilMidnight + 1000); // +1s buffer
  };

  onMount(() => {
    scheduleMidnightRefresh();
  });

  onDestroy(() => {
    if (midnightTimer) clearTimeout(midnightTimer);
  });

  $: sessionCount = $sessions.length;
  $: paneCount = $sessions.reduce((sum, session) => sum + (session.paneCount ?? 0), 0);
  $: compactsToday = $events.filter(
    (event) => event.type === 'compact' && normalizeTimestamp(event.detectedAt) >= todayStartMs
  ).length;
  $: activeMinutes = $dailyStats
    .filter((stat) => normalizeTimestamp(stat.dayStart) >= todayStartMs)
    .reduce((sum, stat) => sum + stat.activeMinutes, 0);
  $: hoursToday = activeMinutes ? (activeMinutes / 60).toFixed(1) : '0.0';
  $: activeSessions = $sessions.filter((session) => session.status === 'active').length;
  $: escalations = $events.filter(
    (event) =>
      event.type === 'escalation' && normalizeTimestamp(event.detectedAt) >= todayStartMs
  ).length;

  // Use centralized status system
  $: connectionStatus = getConnectionStatus($connectionState);
</script>

<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
  <div class="card-compact">
    <p class="label">Sessions</p>
    <p class="mt-3 text-2xl font-semibold text-text-primary">{sessionCount}</p>
    <p class="mt-1 text-xs text-text-subtle">{activeSessions} active</p>
  </div>
  <div class="card-compact">
    <p class="label">Panes</p>
    <p class="mt-3 text-2xl font-semibold text-text-primary">{paneCount}</p>
    <p class="mt-1 text-xs text-text-subtle">Across all sessions</p>
  </div>
  <div class="card-compact">
    <p class="label">Compacts</p>
    <p class="mt-3 text-2xl font-semibold text-text-primary">{compactsToday}</p>
    <p class="mt-1 text-xs text-text-subtle">Today</p>
  </div>
  <div class="card-compact">
    <p class="label">Escalations</p>
    <p class="mt-3 text-2xl font-semibold text-text-primary">{escalations}</p>
    <p class="mt-1 text-xs text-text-subtle">Last 24h</p>
  </div>
  <div class="card-compact">
    <p class="label">Connection</p>
    <div class="mt-3 flex items-center gap-2">
      <span class="badge {connectionStatus.badge}">
        {connectionStatus.label}
      </span>
    </div>
    <p class="mt-2 text-xs text-text-subtle">{hoursToday}h active today</p>
  </div>
</div>
