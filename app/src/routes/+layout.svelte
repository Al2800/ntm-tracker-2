<script lang="ts">
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { goto } from '$app/navigation';
  import { onDestroy, onMount } from 'svelte';
  import { initNotifications, stopNotifications } from '$lib/notifications';
  import { startConnectionLoop, stopConnectionLoop } from '$lib/stores/connection';
  import { selectSession } from '$lib/stores/sessions';
  import { initSettings } from '$lib/stores/settings';
  import '../app.css';

  let unlistenOpenSettings: UnlistenFn | null = null;
  let unlistenOpenSession: UnlistenFn | null = null;
  let unlistenOpenSearch: UnlistenFn | null = null;

  onMount(() => {
    initSettings();
    startConnectionLoop();
    initNotifications();

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

    return () => {
      unlistenOpenSettings?.();
      unlistenOpenSettings = null;
      unlistenOpenSession?.();
      unlistenOpenSession = null;
      unlistenOpenSearch?.();
      unlistenOpenSearch = null;
      stopNotifications();
      stopConnectionLoop();
    };
  });

  onDestroy(() => {
    unlistenOpenSettings?.();
    unlistenOpenSettings = null;
    unlistenOpenSession?.();
    unlistenOpenSession = null;
    unlistenOpenSearch?.();
    unlistenOpenSearch = null;
    stopNotifications();
    stopConnectionLoop();
  });
</script>

<slot />
