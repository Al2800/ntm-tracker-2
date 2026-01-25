<svelte:head>
  <title>NTM Tracker</title>
</svelte:head>

<script lang="ts">
  import { connectionState, lastConnectionError } from '$lib/stores/connection';
</script>

<main class="min-h-screen bg-slate-950 text-slate-100">
  <div class="mx-auto max-w-5xl px-6 py-16">
    {#if $connectionState !== 'connected'}
      <div class="mb-6 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm">
        <div class="flex items-center justify-between gap-3">
          <span class="font-semibold text-amber-200">
            Connection: {$connectionState}
          </span>
          {#if $lastConnectionError}
            <span class="text-amber-100/80">{$lastConnectionError}</span>
          {/if}
        </div>
      </div>
    {/if}
    <p class="text-sm uppercase tracking-[0.3em] text-slate-400">NTM Tracker</p>
    <h1 class="mt-4 text-4xl font-semibold text-white">
      System tray telemetry for your NTM sessions
    </h1>
    <p class="mt-4 max-w-2xl text-lg text-slate-300">
      This UI will surface live sessions, compact events, escalations, and usage
      analytics once the daemon is wired up.
    </p>
    <div class="mt-10 grid gap-4 sm:grid-cols-3">
      <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
        <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Sessions</p>
        <p class="mt-3 text-2xl font-semibold text-white">0 active</p>
      </div>
      <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
        <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Compacts</p>
        <p class="mt-3 text-2xl font-semibold text-white">0 today</p>
      </div>
      <div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
        <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Escalations</p>
        <p class="mt-3 text-2xl font-semibold text-white">None</p>
      </div>
    </div>
  </div>
</main>
