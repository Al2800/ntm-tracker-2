<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { initNotifications, stopNotifications } from '$lib/notifications';
  import { startConnectionLoop, stopConnectionLoop } from '$lib/stores/connection';
  import '../app.css';

  onMount(() => {
    startConnectionLoop();
    initNotifications();
    return () => {
      stopNotifications();
      stopConnectionLoop();
    };
  });

  onDestroy(() => {
    stopNotifications();
    stopConnectionLoop();
  });
</script>

<slot />
