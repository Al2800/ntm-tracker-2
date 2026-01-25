<script lang="ts">
  import { page } from '$app/stores';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { goto } from '$app/navigation';
  import { onDestroy, onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { initNotifications, stopNotifications } from '$lib/notifications';
  import { startConnectionLoop, stopConnectionLoop } from '$lib/stores/connection';
  import { selectSession } from '$lib/stores/sessions';
  import { initSettings, settings, settingsReady } from '$lib/stores/settings';
  import '../app.css';

  // Track all cleanup functions in an array to handle async listener setup
  const cleanupFns: (() => void)[] = [];
  let mounted = true;

  const addCleanup = (fn: () => void) => {
    if (mounted) {
      cleanupFns.push(fn);
    } else {
      // If already unmounted, run cleanup immediately
      fn();
    }
  };

  const cleanup = () => {
    mounted = false;
    for (const fn of cleanupFns) {
      try {
        fn();
      } catch {
        // Ignore cleanup errors
      }
    }
    cleanupFns.length = 0;
    stopNotifications();
    stopConnectionLoop();
  };

  onMount(() => {
    initSettings();
    startConnectionLoop();
    initNotifications();

    const unsubscribeSettingsReady = settingsReady.subscribe((ready) => {
      if (!ready) return;
      const current = get(settings);
      const path = get(page).url.pathname;
      if (!current.firstRunComplete && path !== '/wizard') {
        void goto('/wizard');
      }
    });
    addCleanup(unsubscribeSettingsReady);

    // Set up event listeners with proper cleanup tracking
    void listen('tray:open-settings', () => {
      void goto('/settings');
    }).then((unlisten) => addCleanup(unlisten));

    void listen<string>('tray:open-session', (event) => {
      selectSession(event.payload);
      void goto('/');
    }).then((unlisten) => addCleanup(unlisten));

    void listen('tray:open-search', () => {
      void goto('/?focusSearch=1');
    }).then((unlisten) => addCleanup(unlisten));
  });

  onDestroy(cleanup);
</script>

<slot />
