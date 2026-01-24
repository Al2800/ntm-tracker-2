import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from './types';

export type HealthStatus = 'running' | 'stopped' | 'error';

export interface HealthResponse {
  status: HealthStatus;
  lastError?: string | null;
}

export const daemonStart = () => invoke<void>('daemon_start');
export const daemonStop = () => invoke<void>('daemon_stop');
export const daemonHealth = () => invoke<HealthResponse>('daemon_health');

export const rpcCall = <T = unknown>(method: string, params: unknown = {}) =>
  invoke<T>('rpc_call', { method, params });

export const getSettings = () => invoke<AppSettings>('get_settings');
export const setSettings = (settings: AppSettings) => invoke<void>('set_settings', { settings });

export const exportDiagnostics = (path: string) =>
  invoke<void>('export_diagnostics', { path });

export const getAttachCommand = (paneId: string) =>
  invoke<string>('get_attach_command', { pane_id: paneId });
