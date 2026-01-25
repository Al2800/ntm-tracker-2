<script lang="ts">
  import { onDestroy } from 'svelte';
  import { getAttachCommand, rpcCallWithRetry } from '$lib/tauri';

  export let paneId: string | null = null;

  type OutputPreviewResult = {
    paneId: string;
    content: string;
    lines: number;
    bytes: number;
    truncated: boolean;
    capturedAt: number;
    redacted: boolean;
  };

  const MAX_LINES = 200;
  const AUTO_REFRESH_MS = 5000;

  let preview: OutputPreviewResult | null = null;
  let loading = false;
  let error: string | null = null;
  let autoRefresh = false;
  let intervalHandle: ReturnType<typeof setInterval> | null = null;

  let attachLoading = false;
  let attachCommand: string | null = null;

  const escapeHtml = (value: string) =>
    value
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#39;');

  const colorClassForCode = (code: number) => {
    switch (code) {
      case 30:
        return 'text-slate-100/80';
      case 31:
        return 'text-rose-300';
      case 32:
        return 'text-emerald-300';
      case 33:
        return 'text-amber-300';
      case 34:
        return 'text-sky-300';
      case 35:
        return 'text-fuchsia-300';
      case 36:
        return 'text-teal-300';
      case 37:
        return 'text-slate-100';
      case 90:
        return 'text-slate-400';
      case 91:
        return 'text-rose-200';
      case 92:
        return 'text-emerald-200';
      case 93:
        return 'text-amber-200';
      case 94:
        return 'text-sky-200';
      case 95:
        return 'text-fuchsia-200';
      case 96:
        return 'text-teal-200';
      case 97:
        return 'text-slate-50';
      default:
        return null;
    }
  };

  const renderAnsi = (input: string) => {
    let bold = false;
    let colorClass: string | null = null;

    const wrap = (chunk: string) => {
      if (!chunk) return '';
      const classes = [bold ? 'font-semibold' : null, colorClass].filter(Boolean).join(' ');
      const escaped = escapeHtml(chunk);
      if (!classes) return escaped;
      return `<span class="${classes}">${escaped}</span>`;
    };

    const applyCodes = (codes: string) => {
      const parts = codes.length === 0 ? ['0'] : codes.split(';');
      for (const part of parts) {
        const code = Number.parseInt(part, 10);
        if (Number.isNaN(code)) continue;
        if (code === 0) {
          bold = false;
          colorClass = null;
        } else if (code === 1) {
          bold = true;
        } else if (code === 22) {
          bold = false;
        } else if (code === 39) {
          colorClass = null;
        } else {
          const mapped = colorClassForCode(code);
          if (mapped) colorClass = mapped;
        }
      }
    };

    let output = '';
    let cursor = 0;
    const regex = /\x1b\[([0-9;]*)m/g;
    for (const match of input.matchAll(regex)) {
      const matchIndex = match.index ?? 0;
      output += wrap(input.slice(cursor, matchIndex));
      applyCodes(match[1] ?? '');
      cursor = matchIndex + match[0].length;
    }
    output += wrap(input.slice(cursor));
    return output;
  };

  $: ansiHtml = preview ? renderAnsi(preview.content) : '';
  $: hasContent = (preview?.content ?? '').trim().length > 0;

  const reset = () => {
    preview = null;
    error = null;
    attachCommand = null;
    attachLoading = false;
  };

  const clearAutoRefresh = () => {
    if (!intervalHandle) return;
    clearInterval(intervalHandle);
    intervalHandle = null;
  };

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
    }
  };

  const loadPreview = async () => {
    if (!paneId) return;
    loading = true;
    error = null;
    try {
      preview = await rpcCallWithRetry<OutputPreviewResult>('panes.outputPreview', {
        paneId,
        maxLines: MAX_LINES
      });
    } catch (caught) {
      error = caught instanceof Error ? caught.message : caught ? String(caught) : 'Unable to load preview';
    } finally {
      loading = false;
    }
  };

  const loadAttach = async () => {
    if (!paneId) return;
    attachLoading = true;
    try {
      attachCommand = await getAttachCommand(paneId);
      return attachCommand;
    } catch (caught) {
      error = caught instanceof Error ? caught.message : caught ? String(caught) : 'Unable to load attach command';
    } finally {
      attachLoading = false;
    }
    return null;
  };

  const handleAttach = async () => {
    if (!paneId) return;
    if (attachCommand) {
      await copyText(attachCommand);
      return;
    }
    const command = await loadAttach();
    if (command) {
      await copyText(command);
    }
  };

  const handleToggleAutoRefresh = async (enabled: boolean) => {
    autoRefresh = enabled;
    clearAutoRefresh();
    if (!paneId || !enabled) return;

    if (!preview) {
      await loadPreview();
    }

    intervalHandle = setInterval(() => {
      void loadPreview();
    }, AUTO_REFRESH_MS);
  };

  $: if (!paneId) {
    reset();
    clearAutoRefresh();
    autoRefresh = false;
  } else if (preview && preview.paneId !== paneId) {
    reset();
    clearAutoRefresh();
    autoRefresh = false;
  }

  onDestroy(() => {
    clearAutoRefresh();
  });
</script>

<div class="rounded-2xl border border-slate-800/80 bg-slate-950/50 p-5">
  <div class="flex flex-wrap items-start justify-between gap-4">
    <div>
      <p class="text-sm font-semibold text-white">Output preview</p>
      <p class="mt-1 text-xs text-slate-400">
        Fetched on demand via <code class="font-mono">tmux capture-pane</code>. Redaction is applied server-side.
      </p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <button
        class="rounded-lg border border-slate-700/80 bg-slate-900 px-3 py-2 text-sm text-slate-100 transition hover:border-slate-500 disabled:opacity-40"
        on:click={loadPreview}
        disabled={loading || !paneId}
      >
        {#if preview}
          Refresh
        {:else}
          Load preview
        {/if}
      </button>

      <label class="flex items-center gap-2 text-xs text-slate-300">
        <input
          type="checkbox"
          checked={autoRefresh}
          on:change={(event) => handleToggleAutoRefresh((event.currentTarget as HTMLInputElement).checked)}
          disabled={!paneId}
        />
        Auto-refresh
      </label>

      <button
        class="rounded-lg border border-slate-700/80 bg-slate-900 px-3 py-2 text-sm text-slate-100 transition hover:border-slate-500 disabled:opacity-40"
        on:click={() => preview && copyText(preview.content)}
        disabled={!preview}
      >
        Copy
      </button>

      <button
        class="rounded-lg border border-slate-700/80 bg-slate-900 px-3 py-2 text-sm text-slate-100 transition hover:border-slate-500 disabled:opacity-40"
        on:click={handleAttach}
        disabled={attachLoading || !paneId}
      >
        {attachCommand ? 'Copy attach cmd' : 'Load attach cmd'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="mt-4 rounded-lg border border-rose-500/40 bg-rose-500/10 px-4 py-3 text-sm text-rose-200">
      {error}
    </div>
  {/if}

  {#if !preview}
    <p class="mt-4 text-sm text-slate-300/80">
      Select “Load preview” to fetch the last {MAX_LINES} lines. (This call is redacted and may be truncated.)
    </p>
  {:else}
    <div class="mt-4 flex flex-wrap items-center justify-between gap-3 text-xs text-slate-400">
      <div class="flex flex-wrap items-center gap-3">
        <span>{preview.lines} lines</span>
        <span>{preview.bytes} bytes</span>
        {#if preview.truncated}
          <span class="rounded-full bg-amber-500/20 px-2 py-0.5 text-amber-200">truncated</span>
        {/if}
        {#if preview.redacted}
          <span class="rounded-full bg-slate-800 px-2 py-0.5 text-slate-200">redacted</span>
        {/if}
      </div>
      <span>pane: {preview.paneId}</span>
    </div>

    <div class="mt-3 max-h-80 overflow-auto rounded-lg border border-slate-800 bg-slate-950/60 p-3">
      {#if !hasContent}
        <p class="text-sm text-slate-300/80">
          No output returned. Output capture may be disabled or the pane is empty.
        </p>
      {:else}
        <pre class="whitespace-pre-wrap font-mono text-xs leading-relaxed text-slate-100"
          >{@html ansiHtml}</pre
        >
      {/if}
    </div>
  {/if}
</div>
