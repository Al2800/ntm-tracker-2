<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { startConnectionLoop, stopConnectionLoop } from '$lib/stores/connection';
  import '../app.css';

  onMount(() => {
    startConnectionLoop();
    return () => stopConnectionLoop();
  });

  onDestroy(() => {
    stopConnectionLoop();
  });
</script>

<slot />
