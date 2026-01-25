<!--
  ErrorBanner.svelte
  Error message banner with optional retry action.
  Use for: connection errors, fetch failures, etc.
-->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let message = 'Something went wrong';
  export let details = '';
  export let severity: 'error' | 'warning' | 'info' = 'error';
  export let dismissible = false;
  export let retryable = false;
  export let compact = false;

  const dispatch = createEventDispatcher<{ dismiss: void; retry: void }>();

  let dismissed = false;

  const severityStyles = {
    error: 'border-status-error-ring bg-status-error-muted text-status-error-text',
    warning: 'border-status-warning-ring bg-status-warning-muted text-status-warning-text',
    info: 'border-status-info-ring bg-status-info-muted text-status-info-text'
  };

  const iconPaths = {
    error: '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />',
    warning: '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />',
    info: '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />'
  };

  function handleDismiss() {
    dismissed = true;
    dispatch('dismiss');
  }

  function handleRetry() {
    dispatch('retry');
  }
</script>

{#if !dismissed}
  <div
    class="flex items-start gap-3 rounded-lg border {severityStyles[severity]} animate-fade-in"
    class:p-4={!compact}
    class:p-3={compact}
    role="alert"
  >
    <svg
      class="shrink-0"
      class:h-5={!compact}
      class:w-5={!compact}
      class:h-4={compact}
      class:w-4={compact}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      {@html iconPaths[severity]}
    </svg>

    <div class="min-w-0 flex-1">
      <p class:text-sm={!compact} class:text-xs={compact} class="font-medium">
        {message}
      </p>
      {#if details}
        <p class="mt-1 text-2xs opacity-80">{details}</p>
      {/if}

      {#if retryable || $$slots.actions}
        <div class="mt-2 flex flex-wrap gap-2">
          {#if retryable}
            <button
              type="button"
              class="btn btn-sm btn-secondary"
              on:click={handleRetry}
            >
              Try again
            </button>
          {/if}
          <slot name="actions" />
        </div>
      {/if}
    </div>

    {#if dismissible}
      <button
        type="button"
        class="shrink-0 rounded p-1 opacity-60 transition-opacity hover:opacity-100"
        on:click={handleDismiss}
        aria-label="Dismiss"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    {/if}
  </div>
{/if}
