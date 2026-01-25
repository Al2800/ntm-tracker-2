<!--
  DashboardLayout.svelte
  Main layout shell providing sidebar + command bar + content structure.
  See docs/information-architecture.md for design rationale.
-->
<script lang="ts">
  import { page } from '$app/stores';

  // Check if we're in compact/tray mode
  $: compactMode =
    $page.url.searchParams.get('view') === 'compact' ||
    $page.url.searchParams.get('compact') === '1';
</script>

<div class="layout-root min-h-screen bg-surface-base text-text-primary">
  <!-- Skip link for keyboard navigation -->
  <a href="#main-content" class="skip-link">Skip to main content</a>

  <!-- Background gradient -->
  <div class="pointer-events-none fixed inset-0 bg-gradient-hero" aria-hidden="true"></div>

  {#if compactMode}
    <!-- Compact/Tray Mode: Single column, no sidebar -->
    <div class="relative flex h-screen flex-col">
      <!-- Compact header slot -->
      <header class="shrink-0 border-b border-border p-3">
        <slot name="compact-header" />
      </header>

      <!-- Compact content -->
      <main class="flex-1 overflow-y-auto p-3">
        <slot name="compact-content" />
      </main>

      <!-- Compact footer/actions -->
      <footer class="shrink-0 border-t border-border p-3">
        <slot name="compact-footer" />
      </footer>
    </div>
  {:else}
    <!-- Full Window Mode: Sidebar + Main -->
    <div class="relative flex h-screen flex-col">
      <!-- Top Command Bar -->
      <header class="shrink-0 border-b border-border bg-surface-raised/50 backdrop-blur-sm">
        <slot name="command-bar" />
      </header>

      <!-- Main content area with sidebar -->
      <div class="flex flex-1 overflow-hidden">
        <!-- Sidebar -->
        <aside
          class="hidden w-72 shrink-0 flex-col border-r border-border bg-surface-raised/30 lg:flex xl:w-80"
          role="navigation"
          aria-label="Sessions navigation"
        >
          <div class="flex-1 overflow-y-auto">
            <slot name="sidebar" />
          </div>
          <!-- Sidebar footer (quick stats) -->
          <div class="shrink-0 border-t border-border p-4">
            <slot name="sidebar-footer" />
          </div>
        </aside>

        <!-- Main content -->
        <main id="main-content" class="flex flex-1 flex-col overflow-hidden" tabindex="-1">
          <!-- Primary content area (focus panel) -->
          <div class="flex-1 overflow-y-auto">
            <div class="mx-auto max-w-5xl p-6">
              <slot name="focus" />
            </div>
          </div>
        </main>

        <!-- Right panel (insights) - hidden on smaller screens -->
        <aside
          class="hidden w-80 shrink-0 flex-col border-l border-border bg-surface-raised/30 xl:flex 2xl:w-96"
          aria-label="Activity insights"
        >
          <div class="flex-1 overflow-y-auto p-4">
            <slot name="insights" />
          </div>
        </aside>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Ensure full viewport coverage */
  .layout-root {
    position: relative;
    isolation: isolate;
  }

  /* Scrollbar styling for sidebar/panels */
  aside :global(::-webkit-scrollbar) {
    width: 6px;
  }

  aside :global(::-webkit-scrollbar-track) {
    background: transparent;
  }

  aside :global(::-webkit-scrollbar-thumb) {
    background: rgb(51 65 85 / 0.5);
    border-radius: 3px;
  }

  aside :global(::-webkit-scrollbar-thumb:hover) {
    background: rgb(51 65 85 / 0.8);
  }
</style>
