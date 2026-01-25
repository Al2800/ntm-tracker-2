<!--
  CommandBar.svelte
  Top navigation bar with search, connection status, and quick actions.
  See docs/information-architecture.md for design rationale.
-->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { connectionState, lastConnectionError } from '$lib/stores/connection';
  import { settings, updateSettings } from '$lib/stores/settings';
  import { events } from '$lib/stores/events';

  export let searchValue = '';
  let searchInput: HTMLInputElement;

  // Connection status styling
  const connectionLabel: Record<string, string> = {
    connected: 'Connected',
    connecting: 'Connecting',
    reconnecting: 'Reconnecting',
    degraded: 'Degraded',
    disconnected: 'Disconnected'
  };

  // Pending escalations count for notification badge
  $: pendingCount = $events.filter(
    (e) => e.eventType === 'escalation' && (e.status ?? 'pending') === 'pending'
  ).length;

  // Handle Ctrl+K to focus search
  onMount(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) {
        e.preventDefault();
        searchInput?.focus();
      }
    };
    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  function toggleNotifications() {
    updateSettings({ showNotifications: !$settings.showNotifications });
  }
</script>

<div class="flex h-14 items-center justify-between gap-4 px-4">
  <!-- Left: Brand -->
  <div class="flex items-center gap-4">
    <button
      type="button"
      class="flex items-center gap-2 text-text-primary hover:text-accent transition-colors focus-ring rounded"
      on:click={() => goto('/')}
      aria-label="Go to NTM Tracker dashboard"
    >
      <span class="label-sm">NTM Tracker</span>
    </button>
  </div>

  <!-- Center: Search -->
  <div class="flex flex-1 justify-center max-w-xl">
    <div class="relative w-full">
      <input
        bind:this={searchInput}
        bind:value={searchValue}
        type="text"
        class="input w-full pl-10 pr-4"
        placeholder="Search sessions… (Ctrl+K)"
        aria-label="Search sessions"
        role="searchbox"
      />
      <svg
        class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-text-subtle"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
        />
      </svg>
    </div>
  </div>

  <!-- Right: Status + Actions -->
  <div class="flex items-center gap-3">
    <!-- Notification toggle -->
    <button
      type="button"
      class="relative btn btn-ghost px-2"
      on:click={toggleNotifications}
      aria-label={$settings.showNotifications ? 'Mute notifications' : 'Enable notifications'}
    >
      {#if $settings.showNotifications}
        <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
        </svg>
      {:else}
        <svg class="h-5 w-5 text-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9M3 3l18 18" />
        </svg>
      {/if}
      {#if pendingCount > 0}
        <span class="absolute -right-1 -top-1 flex h-4 w-4 items-center justify-center rounded-full bg-status-warning text-[10px] font-bold text-text-inverse">
          {pendingCount > 9 ? '9+' : pendingCount}
        </span>
      {/if}
    </button>

    <!-- Settings -->
    <button
      type="button"
      class="btn btn-ghost px-2"
      on:click={() => goto('/settings')}
      aria-label="Settings"
    >
      <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    </button>

    <!-- Connection status badge -->
    <div class="flex items-center gap-2" role="status" aria-live="polite">
      <span
        class="badge"
        class:badge-success={$connectionState === 'connected'}
        class:badge-info={$connectionState === 'connecting'}
        class:badge-warning={$connectionState === 'reconnecting'}
        class:badge-error={$connectionState === 'degraded'}
        class:badge-neutral={$connectionState === 'disconnected'}
        title={$lastConnectionError || undefined}
        aria-label="Connection status: {connectionLabel[$connectionState] ?? 'Unknown'}"
      >
        {connectionLabel[$connectionState] ?? 'Unknown'}
      </span>
    </div>
  </div>
</div>
