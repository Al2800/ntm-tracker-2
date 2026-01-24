<script lang="ts">
  import { sessions } from '../stores/sessions';
  import { events } from '../stores/events';
  import { dailyStats } from '../stores/stats';

  const normalizeTimestamp = (value: number) => (value < 1_000_000_000_000 ? value * 1000 : value);

  const todayStartMs = (() => {
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    return now.getTime();
  })();

  $: sessionCount = $sessions.length;
  $: paneCount = $sessions.reduce((sum, session) => sum + (session.paneCount ?? 0), 0);
  $: compactsToday = $events.filter(
    (event) => event.type === 'compact' && normalizeTimestamp(event.detectedAt) >= todayStartMs
  ).length;
  $: activeMinutes = $dailyStats
    .filter((stat) => normalizeTimestamp(stat.dayStart) >= todayStartMs)
    .reduce((sum, stat) => sum + stat.activeMinutes, 0);
  $: hoursToday = activeMinutes ? (activeMinutes / 60).toFixed(1) : '0.0';
</script>

<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
  <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Sessions</p>
    <p class="mt-3 text-2xl font-semibold text-white">{sessionCount}</p>
  </div>
  <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Panes</p>
    <p class="mt-3 text-2xl font-semibold text-white">{paneCount}</p>
  </div>
  <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Compacts</p>
    <p class="mt-3 text-2xl font-semibold text-white">{compactsToday}</p>
  </div>
  <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Today</p>
    <p class="mt-3 text-2xl font-semibold text-white">{hoursToday}h</p>
  </div>
</div>
