import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from './types';

export type HealthStatus = 'running' | 'stopped' | 'error';

export interface HealthResponse {
  status: HealthStatus;
  lastError?: string | null;
}

export type RetryOptions = {
  maxRetries?: number;
  baseDelayMs?: number;
  backoffFactor?: number;
  idempotent?: boolean;
  retryable?: (error: unknown) => boolean;
};

export const daemonStart = () => invoke<void>('daemon_start');
export const daemonStop = () => invoke<void>('daemon_stop');
export const daemonHealth = () => invoke<HealthResponse>('daemon_health');

export const rpcCall = <T = unknown>(method: string, params: unknown = {}) =>
  invoke<T>('rpc_call', { method, params });

const defaultRetryable = (error: unknown) => {
  const message =
    error instanceof Error ? error.message : error ? String(error) : '';
  return /timeout|timed out|connection|disconnected|reset|503|unavailable/i.test(message);
};

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const rpcCallWithRetry = async <T = unknown>(
  method: string,
  params: unknown = {},
  options: RetryOptions = {}
) => {
  const {
    maxRetries = 3,
    baseDelayMs = 200,
    backoffFactor = 2,
    idempotent = true,
    retryable = defaultRetryable
  } = options;

  let attempt = 0;
  while (true) {
    try {
      return await rpcCall<T>(method, params);
    } catch (error) {
      if (!idempotent || attempt >= maxRetries || !retryable(error)) {
        throw error;
      }
      const delay = baseDelayMs * backoffFactor ** attempt;
      attempt += 1;
      await sleep(delay);
    }
  }
};

export const getSettings = () => invoke<AppSettings>('get_settings');
export const setSettings = (settings: AppSettings) => invoke<void>('set_settings', { settings });

export const exportDiagnostics = (path: string) =>
  invoke<void>('export_diagnostics', { path });

export const getAttachCommand = (paneId: string) =>
  invoke<string>('get_attach_command', { pane_id: paneId });
