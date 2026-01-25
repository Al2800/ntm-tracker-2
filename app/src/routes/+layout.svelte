<script lang="ts">
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { goto } from '$app/navigation';
  import { onDestroy, onMount } from 'svelte';
  import { initNotifications, stopNotifications } from '$lib/notifications';
  import { startConnectionLoop, stopConnectionLoop } from '$lib/stores/connection';
  import { initSettings } from '$lib/stores/settings';
  import '../app.css';

  let unlistenOpenSettings: UnlistenFn | null = null;

  onMount(() => {
    initSettings();
    startConnectionLoop();
    initNotifications();

    void listen('tray:open-settings', () => {
      void goto('/settings');
    }).then((unlisten) => {
      unlistenOpenSettings = unlisten;
    });

    return () => {
      unlistenOpenSettings?.();
      unlistenOpenSettings = null;
      stopNotifications();
      stopConnectionLoop();
    };
  });

  onDestroy(() => {
    unlistenOpenSettings?.();
    unlistenOpenSettings = null;
    stopNotifications();
    stopConnectionLoop();
  });
</script>

<slot />
