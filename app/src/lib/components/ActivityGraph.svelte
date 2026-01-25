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
  $: totalActive = series.reduce((sum, e) => sum + e.activeMinutes, 0);
  $: totalCompacts = series.reduce((sum, e) => sum + e.totalCompacts, 0);

  const labelFor = (bucket: number) =>
    new Date(bucket).toLocaleTimeString([], { hour: '2-digit' });

  const barHeight = (value: number) => Math.max(6, (value / maxValue) * height);

  const isNowBucket = (bucket: number) => {
    const now = new Date();
    return new Date(bucket).getHours() === now.getHours();
  };
</script>

<div class="card">
  <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
    <div>
      <h3 class="label">Activity (24h)</h3>
      <p class="mt-1 text-xs text-text-subtle">Active minutes + compacts per hour</p>
    </div>
    {#if showLegend}
      <div class="flex flex-wrap items-center gap-2">
        <button
          class={`chip ${showActive ? 'chip-success' : 'chip-inactive'}`}
          on:click={() => (showActive = !showActive)}
          type="button"
        >
          <span class="status-dot status-dot-success"></span>
          Active
        </button>
        <button
          class={`chip ${showCompacts ? 'chip-warning' : 'chip-inactive'}`}
          on:click={() => (showCompacts = !showCompacts)}
          type="button"
        >
          <span class="status-dot status-dot-warning"></span>
          Compacts
        </button>
      </div>
    {/if}
  </div>

  <!-- Summary stats -->
  <div class="mb-4 grid grid-cols-2 gap-3">
    <div class="rounded-lg border border-border bg-surface-base px-3 py-2 text-center">
      <p class="text-lg font-bold text-status-success">{totalActive}</p>
      <p class="label-sm">Active mins</p>
    </div>
    <div class="rounded-lg border border-border bg-surface-base px-3 py-2 text-center">
      <p class="text-lg font-bold text-status-warning">{totalCompacts}</p>
      <p class="label-sm">Compacts</p>
    </div>
  </div>

  <div
    class="relative grid items-end gap-1 rounded-xl border border-border bg-surface-base px-2 py-3"
    style="grid-template-columns: repeat(24, minmax(0, 1fr));"
  >
    {#each series as entry (entry.bucket)}
      <div class="flex flex-col items-center gap-2">
        <div class="flex items-end gap-0.5">
          {#if showActive}
            <div
              class={`w-1.5 rounded-full bg-status-success ${
                isNowBucket(entry.bucket) ? 'shadow-glow-success' : ''
              }`}
              style={`height: ${barHeight(entry.activeMinutes)}px`}
              title={`Active: ${entry.activeMinutes}m | Compacts: ${entry.totalCompacts}`}
            ></div>
          {/if}
          {#if showCompacts}
            <div
              class={`w-1.5 rounded-full bg-status-warning ${
                isNowBucket(entry.bucket) ? 'shadow-glow-warning' : ''
              }`}
              style={`height: ${barHeight(entry.totalCompacts)}px`}
              title={`Active: ${entry.activeMinutes}m | Compacts: ${entry.totalCompacts}`}
            ></div>
          {/if}
        </div>
        <span class="text-[9px] text-text-muted">{labelFor(entry.bucket)}</span>
      </div>
    {/each}
  </div>
</div>
