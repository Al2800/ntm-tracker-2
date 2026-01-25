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

  let unlistenOpenSettings: UnlistenFn | null = null;
  let unlistenOpenSession: UnlistenFn | null = null;
  let unlistenOpenSearch: UnlistenFn | null = null;
  let unsubscribeSettingsReady: (() => void) | null = null;

  const cleanup = () => {
    unlistenOpenSettings?.();
    unlistenOpenSettings = null;
    unlistenOpenSession?.();
    unlistenOpenSession = null;
    unlistenOpenSearch?.();
    unlistenOpenSearch = null;
    unsubscribeSettingsReady?.();
    unsubscribeSettingsReady = null;
    stopNotifications();
    stopConnectionLoop();
  };

  onMount(() => {
    initSettings();
    startConnectionLoop();
    initNotifications();

    unsubscribeSettingsReady = settingsReady.subscribe((ready) => {
      if (!ready) return;
      const current = get(settings);
      const path = get(page).url.pathname;
      if (!current.firstRunComplete && path !== '/wizard') {
        void goto('/wizard');
      }
    });

    void listen('tray:open-settings', () => {
      void goto('/settings');
    }).then((unlisten) => {
      unlistenOpenSettings = unlisten;
    });

    void listen<string>('tray:open-session', (event) => {
      selectSession(event.payload);
      void goto('/');
    }).then((unlisten) => {
      unlistenOpenSession = unlisten;
    });

    void listen('tray:open-search', () => {
      void goto('/?focusSearch=1');
    }).then((unlisten) => {
      unlistenOpenSearch = unlisten;
    });
  });

  // Use onDestroy for cleanup - more reliable than onMount return in Svelte
  onDestroy(cleanup);
</script>

<slot />
