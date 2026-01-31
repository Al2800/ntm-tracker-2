export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'degraded';

export interface Pane {
  paneId: string;
  sessionId: string;
  paneIndex: number;
  status: 'active' | 'idle' | 'waiting' | 'ended' | 'unknown';
  agentType?: string;
  lastActivityAt?: number;
  currentCommand?: string;
  tmuxPaneId?: string | null;
  tmuxWindowId?: string | null;
  tmuxPanePid?: number | null;
}

export interface Session {
  sessionId: string;
  name: string;
  status: 'active' | 'idle' | 'waiting' | 'ended' | 'unknown';
  paneCount: number;
  panes?: Pane[];
  lastSeenAt?: number;
  tmuxSessionId?: string | null;
  sourceId?: string;
}

export interface TrackerEvent {
  id: number;
  sessionId: string;
  paneId: string;
  eventType: 'compact' | 'escalation' | 'pane.status' | 'session.status';
  detectedAt: number;
  severity?: 'info' | 'warn' | 'error';
  message?: string;
  status?: 'pending' | 'resolved' | 'dismissed';
}

export interface HourlyStats {
  hourStart: number;
  sessionId: string;
  totalCompacts: number;
  activeMinutes: number;
  estimatedTokens: number;
}

export interface DailyStats {
  dayStart: number;
  sessionId: string;
  totalCompacts: number;
  activeMinutes: number;
  estimatedTokens: number;
}

export interface AppSettings {
  transport: 'wsl-stdio' | 'ws' | 'http';
  wslDistro: string | null;
  reconnectIntervalMs: number;
  autostartEnabled: boolean;
  showNotifications: boolean;
  notifyOnCompact: boolean;
  notifyOnEscalation: boolean;
  quietHoursStart: number;
  quietHoursEnd: number;
  notificationMaxPerHour: number;
  theme: 'system' | 'light' | 'dark';
  debugMode: boolean;
  logLevel: 'trace' | 'debug' | 'info' | 'warn' | 'error';
  firstRunComplete: boolean;
}
