<script lang="ts">
  import { sessions } from '../stores/sessions';
  import { events } from '../stores/events';
  import { dailyStats } from '../stores/stats';
  import { connectionState } from '../stores/connection';

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
  $: activeSessions = $sessions.filter((session) => session.status === 'active').length;
  $: escalations = $events.filter(
    (event) =>
      event.type === 'escalation' && normalizeTimestamp(event.detectedAt) >= todayStartMs
  ).length;

  const connectionLabel: Record<string, string> = {
    connected: 'Connected',
    connecting: 'Connecting',
    reconnecting: 'Reconnecting',
    degraded: 'Degraded',
    disconnected: 'Disconnected'
  };

  const connectionClass: Record<string, string> = {
    connected: 'bg-emerald-500/15 text-emerald-200 ring-1 ring-emerald-400/40',
    connecting: 'bg-sky-500/15 text-sky-200 ring-1 ring-sky-400/40',
    reconnecting: 'bg-amber-500/15 text-amber-200 ring-1 ring-amber-400/40',
    degraded: 'bg-rose-500/15 text-rose-200 ring-1 ring-rose-400/40',
    disconnected: 'bg-slate-500/15 text-slate-200 ring-1 ring-slate-500/40'
  };
</script>

<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
  <div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Sessions</p>
    <p class="mt-3 text-2xl font-semibold text-white">{sessionCount}</p>
    <p class="mt-1 text-xs text-slate-500">{activeSessions} active</p>
  </div>
  <div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Panes</p>
    <p class="mt-3 text-2xl font-semibold text-white">{paneCount}</p>
    <p class="mt-1 text-xs text-slate-500">Across all sessions</p>
  </div>
  <div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Compacts</p>
    <p class="mt-3 text-2xl font-semibold text-white">{compactsToday}</p>
    <p class="mt-1 text-xs text-slate-500">Today</p>
  </div>
  <div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Escalations</p>
    <p class="mt-3 text-2xl font-semibold text-white">{escalations}</p>
    <p class="mt-1 text-xs text-slate-500">Last 24h</p>
  </div>
  <div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Connection</p>
    <div class="mt-3 flex items-center gap-2 text-sm text-slate-200">
      <span class={`rounded-full px-3 py-1 text-xs ${connectionClass[$connectionState]}`}>
        {connectionLabel[$connectionState]}
      </span>
    </div>
    <p class="mt-2 text-xs text-slate-500">{hoursToday}h active today</p>
  </div>
</div>
