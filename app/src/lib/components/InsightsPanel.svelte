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
    (e) => e.eventType === 'escalation' && (e.status ?? 'pending') === 'pending'
  ).length;
</script>

<div class="space-y-4">
  <!-- Section: Activity -->
  <section aria-labelledby="activity-heading">
    <button
      type="button"
      class="flex w-full items-center justify-between py-1 text-left focus-ring rounded"
      on:click={() => showActivity = !showActivity}
      aria-expanded={showActivity}
      aria-controls="activity-content"
    >
      <h2 id="activity-heading" class="label">Activity</h2>
      <span class="text-text-subtle transition-transform" class:rotate-180={!showActivity} aria-hidden="true">
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </span>
    </button>
    {#if showActivity}
      <div id="activity-content" class="mt-2 animate-fade-in">
        <ActivityGraph height={120} showLegend={false} />
      </div>
    {/if}
  </section>

  <!-- Section: Escalations -->
  <section aria-labelledby="escalations-heading">
    <button
      type="button"
      class="flex w-full items-center justify-between py-1 text-left focus-ring rounded"
      on:click={() => showEscalations = !showEscalations}
      aria-expanded={showEscalations}
      aria-controls="escalations-content"
    >
      <div class="flex items-center gap-2">
        <h2 id="escalations-heading" class="label">Escalations</h2>
        {#if pendingCount > 0}
          <span class="badge badge-error text-2xs py-0 px-1.5" aria-label="{pendingCount} pending escalations">
            {pendingCount}
          </span>
        {/if}
      </div>
      <span class="text-text-subtle transition-transform" class:rotate-180={!showEscalations} aria-hidden="true">
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </span>
    </button>
    {#if showEscalations}
      <div id="escalations-content" class="mt-2 animate-fade-in">
        <EscalationPanel />
      </div>
    {/if}
  </section>

  <!-- Section: Timeline -->
  <section aria-labelledby="timeline-heading">
    <button
      type="button"
      class="flex w-full items-center justify-between py-1 text-left focus-ring rounded"
      on:click={() => showTimeline = !showTimeline}
      aria-expanded={showTimeline}
      aria-controls="timeline-content"
    >
      <h2 id="timeline-heading" class="label">Timeline</h2>
      <span class="text-text-subtle transition-transform" class:rotate-180={!showTimeline} aria-hidden="true">
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </span>
    </button>
    {#if showTimeline}
      <div id="timeline-content" class="mt-2 animate-fade-in">
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
