export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'degraded';

export interface Pane {
  paneUid: string;
  sessionUid: string;
  index: number;
  status: 'active' | 'idle' | 'waiting' | 'ended' | 'unknown';
  agentType?: string;
  lastActivityAt?: number;
  currentCommand?: string;
}

export interface Session {
  sessionUid: string;
  name: string;
  status: 'active' | 'idle' | 'ended' | 'unknown';
  paneCount: number;
  panes?: Pane[];
  lastSeenAt?: number;
}

export interface TrackerEvent {
  id: number;
  sessionUid: string;
  paneUid: string;
  type: 'compact' | 'escalation' | 'pane.status' | 'session.status';
  detectedAt: number;
  severity?: 'info' | 'warn' | 'error';
  message?: string;
  status?: 'pending' | 'resolved' | 'dismissed';
}

export interface HourlyStats {
  hourStart: number;
  sessionUid: string;
  totalCompacts: number;
  activeMinutes: number;
  estimatedTokens: number;
}

export interface DailyStats {
  dayStart: number;
  sessionUid: string;
  totalCompacts: number;
  activeMinutes: number;
  estimatedTokens: number;
}

export interface AppSettings {
  transport: 'wsl-stdio' | 'ws' | 'http';
  reconnectIntervalMs: number;
  autostartEnabled: boolean;
  showNotifications: boolean;
  notifyOnCompact: boolean;
  notifyOnEscalation: boolean;
  theme: 'system' | 'light' | 'dark';
}
