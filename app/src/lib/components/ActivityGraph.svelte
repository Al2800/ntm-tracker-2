<script lang="ts">
  import { hourlyStats } from '../stores/stats';

  export let height = 140;
  export let showLegend = true;

  let showActive = true;
  let showCompacts = true;

  const normalizeTimestamp = (value: number) => (value < 1_000_000_000_000 ? value * 1000 : value);

  const hourBuckets = () => {
    const now = new Date();
    now.setMinutes(0, 0, 0);
    return Array.from({ length: 24 }, (_, index) => {
      const stamp = now.getTime() - (23 - index) * 60 * 60 * 1000;
      return stamp;
    });
  };

  $: buckets = hourBuckets();
  $: statsByHour = new Map(
    $hourlyStats.map((stat) => [normalizeTimestamp(stat.hourStart), stat])
  );
  $: series = buckets.map((bucket) => {
    const stat = statsByHour.get(bucket);
    return {
      bucket,
      activeMinutes: stat?.activeMinutes ?? 0,
      totalCompacts: stat?.totalCompacts ?? 0,
      stat,
    };
  });
  $: maxValue = Math.max(
    1,
    ...series.map((entry) => entry.activeMinutes),
    ...series.map((entry) => entry.totalCompacts)
  );

  const labelFor = (bucket: number) =>
    new Date(bucket).toLocaleTimeString([], { hour: '2-digit' });

  const barHeight = (value: number) => Math.max(6, (value / maxValue) * height);

  const isNowBucket = (bucket: number) => {
    const now = new Date();
    return new Date(bucket).getHours() === now.getHours();
  };
</script>

<div class="rounded-2xl border border-slate-800/80 bg-slate-900/60 p-5">
  <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
    <div>
      <h3 class="text-sm font-semibold uppercase tracking-[0.2em] text-slate-300">
        Activity (24h)
      </h3>
      <p class="mt-1 text-xs text-slate-500">Active minutes + compacts per hour</p>
    </div>
    {#if showLegend}
      <div class="flex flex-wrap items-center gap-2 text-xs text-slate-400">
        <button
          class={`flex items-center gap-2 rounded-full border px-3 py-1 transition ${
            showActive ? 'border-emerald-400/60 text-emerald-200' : 'border-slate-700 text-slate-500'
          }`}
          on:click={() => (showActive = !showActive)}
          type="button"
        >
          <span class="h-2 w-2 rounded-full bg-emerald-400/70"></span>
          Active
        </button>
        <button
          class={`flex items-center gap-2 rounded-full border px-3 py-1 transition ${
            showCompacts ? 'border-amber-400/60 text-amber-200' : 'border-slate-700 text-slate-500'
          }`}
          on:click={() => (showCompacts = !showCompacts)}
          type="button"
        >
          <span class="h-2 w-2 rounded-full bg-amber-400/70"></span>
          Compacts
        </button>
      </div>
    {/if}
  </div>

  <div
    class="relative grid items-end gap-1 rounded-xl border border-slate-800/60 bg-slate-950/60 px-2 py-3"
    style="grid-template-columns: repeat(24, minmax(0, 1fr));"
  >
    {#each series as entry (entry.bucket)}
      <div class="flex flex-col items-center gap-2">
        <div class="flex items-end gap-1">
          {#if showActive}
            <div
              class={`w-2 rounded-full bg-emerald-400/70 ${
                isNowBucket(entry.bucket) ? 'shadow-[0_0_12px_rgba(52,211,153,0.45)]' : ''
              }`}
              style={`height: ${barHeight(entry.activeMinutes)}px`}
              title={`Active: ${entry.activeMinutes}m | Compacts: ${entry.totalCompacts}`}
            ></div>
          {/if}
          {#if showCompacts}
            <div
              class={`w-2 rounded-full bg-amber-400/70 ${
                isNowBucket(entry.bucket) ? 'shadow-[0_0_12px_rgba(251,191,36,0.45)]' : ''
              }`}
              style={`height: ${barHeight(entry.totalCompacts)}px`}
              title={`Active: ${entry.activeMinutes}m | Compacts: ${entry.totalCompacts}`}
            ></div>
          {/if}
        </div>
        <span class="text-[10px] text-slate-500">{labelFor(entry.bucket)}</span>
      </div>
    {/each}
  </div>
</div>
