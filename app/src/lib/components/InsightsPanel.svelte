<!--
  InsightsPanel.svelte
  Sidebar panel containing activity graph, escalation inbox, and timeline.
  Designed for the "insights" slot in DashboardLayout.
  See docs/information-architecture.md for design rationale.
-->
<script lang="ts">
  import { events } from '$lib/stores/events';
  import ActivityGraph from './ActivityGraph.svelte';
  import EscalationPanel from './EscalationPanel.svelte';
  import TimelinePanel from './TimelinePanel.svelte';

  // Collapsible sections state
  let showActivity = true;
  let showEscalations = true;
  let showTimeline = true;

  // Pending escalations count for header
  $: pendingCount = $events.filter(
    (e) => e.type === 'escalation' && (e.status ?? 'pending') === 'pending'
  ).length;
</script>

<div class="space-y-4">
  <!-- Section: Activity -->
  <section>
    <button
      type="button"
      class="flex w-full items-center justify-between py-1 text-left"
      on:click={() => showActivity = !showActivity}
    >
      <h2 class="label">Activity</h2>
      <span class="text-text-subtle transition-transform" class:rotate-180={!showActivity}>
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </span>
    </button>
    {#if showActivity}
      <div class="mt-2 animate-fade-in">
        <ActivityGraph height={120} showLegend={false} />
      </div>
    {/if}
  </section>

  <!-- Section: Escalations -->
  <section>
    <button
      type="button"
      class="flex w-full items-center justify-between py-1 text-left"
      on:click={() => showEscalations = !showEscalations}
    >
      <div class="flex items-center gap-2">
        <h2 class="label">Escalations</h2>
        {#if pendingCount > 0}
          <span class="badge badge-error text-2xs py-0 px-1.5">
            {pendingCount}
          </span>
        {/if}
      </div>
      <span class="text-text-subtle transition-transform" class:rotate-180={!showEscalations}>
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </span>
    </button>
    {#if showEscalations}
      <div class="mt-2 animate-fade-in">
        <EscalationPanel />
      </div>
    {/if}
  </section>

  <!-- Section: Timeline -->
  <section>
    <button
      type="button"
      class="flex w-full items-center justify-between py-1 text-left"
      on:click={() => showTimeline = !showTimeline}
    >
      <h2 class="label">Timeline</h2>
      <span class="text-text-subtle transition-transform" class:rotate-180={!showTimeline}>
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </span>
    </button>
    {#if showTimeline}
      <div class="mt-2 animate-fade-in">
        <TimelinePanel limit={15} />
      </div>
    {/if}
  </section>
</div>

<style>
  .rotate-180 {
    transform: rotate(180deg);
  }
</style>
