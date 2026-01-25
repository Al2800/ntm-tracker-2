/**
 * Status Badge System - Centralized status definitions
 *
 * This module defines the canonical mapping for all status types in NTM Tracker.
 * Use these constants to ensure consistent status representation across the UI.
 *
 * @see docs/design-tokens.md for color token usage
 * @see app.css for badge component classes
 */

// =============================================================================
// CONNECTION STATUS
// =============================================================================

export type ConnectionStatus = 'connected' | 'connecting' | 'reconnecting' | 'degraded' | 'disconnected';

export const CONNECTION_STATUS: Record<ConnectionStatus, {
  label: string;
  badge: string;
  dot: string;
  description: string;
}> = {
  connected: {
    label: 'Connected',
    badge: 'badge-success',
    dot: 'status-dot-success',
    description: 'Daemon is connected and responding',
  },
  connecting: {
    label: 'Connecting',
    badge: 'badge-info',
    dot: 'status-dot-info status-dot-pulse',
    description: 'Establishing connection to daemon',
  },
  reconnecting: {
    label: 'Reconnecting',
    badge: 'badge-warning',
    dot: 'status-dot-warning status-dot-pulse',
    description: 'Connection lost, attempting to reconnect',
  },
  degraded: {
    label: 'Degraded',
    badge: 'badge-error',
    dot: 'status-dot-error',
    description: 'Connection unstable or partial',
  },
  disconnected: {
    label: 'Disconnected',
    badge: 'badge-neutral',
    dot: 'status-dot-neutral',
    description: 'No connection to daemon',
  },
};

// =============================================================================
// SESSION STATUS
// =============================================================================

export type SessionStatus = 'active' | 'idle' | 'waiting' | 'ended' | 'unknown';

export const SESSION_STATUS: Record<SessionStatus, {
  label: string;
  badge: string;
  dot: string;
  rank: number;
  description: string;
}> = {
  active: {
    label: 'Active',
    badge: 'badge-success',
    dot: 'status-dot-success',
    rank: 0,
    description: 'Session has recent activity',
  },
  idle: {
    label: 'Idle',
    badge: 'badge-warning',
    dot: 'status-dot-warning',
    rank: 1,
    description: 'Session has no recent activity',
  },
  waiting: {
    label: 'Waiting',
    badge: 'badge-info',
    dot: 'status-dot-info status-dot-pulse',
    rank: 2,
    description: 'Session is waiting for input',
  },
  ended: {
    label: 'Ended',
    badge: 'badge-neutral',
    dot: 'status-dot-neutral',
    rank: 3,
    description: 'Session has terminated',
  },
  unknown: {
    label: 'Unknown',
    badge: 'badge-neutral',
    dot: 'status-dot-neutral',
    rank: 4,
    description: 'Session status cannot be determined',
  },
};

// =============================================================================
// PANE STATUS
// =============================================================================

export type PaneStatus = 'active' | 'idle' | 'waiting' | 'ended' | 'unknown';

export const PANE_STATUS: Record<PaneStatus, {
  label: string;
  badge: string;
  dot: string;
  rank: number;
  description: string;
}> = {
  active: {
    label: 'Active',
    badge: 'badge-success',
    dot: 'status-dot-success',
    rank: 0,
    description: 'Pane has recent output',
  },
  idle: {
    label: 'Idle',
    badge: 'badge-warning',
    dot: 'status-dot-warning',
    rank: 1,
    description: 'Pane has no recent output',
  },
  waiting: {
    label: 'Waiting',
    badge: 'badge-info',
    dot: 'status-dot-info status-dot-pulse',
    rank: 2,
    description: 'Pane is waiting at a prompt',
  },
  ended: {
    label: 'Ended',
    badge: 'badge-neutral',
    dot: 'status-dot-neutral',
    rank: 3,
    description: 'Pane process has exited',
  },
  unknown: {
    label: 'Unknown',
    badge: 'badge-neutral',
    dot: 'status-dot-neutral',
    rank: 4,
    description: 'Pane status cannot be determined',
  },
};

// =============================================================================
// ESCALATION SEVERITY
// =============================================================================

export type EscalationSeverity = 'critical' | 'high' | 'medium' | 'low' | 'info';

export const ESCALATION_SEVERITY: Record<EscalationSeverity, {
  label: string;
  badge: string;
  dot: string;
  chip: string;
  rank: number;
  description: string;
}> = {
  critical: {
    label: 'Critical',
    badge: 'badge-error',
    dot: 'status-dot-error status-dot-pulse',
    chip: 'chip-error',
    rank: 0,
    description: 'Immediate attention required',
  },
  high: {
    label: 'High',
    badge: 'badge-error',
    dot: 'status-dot-error',
    chip: 'chip-error',
    rank: 1,
    description: 'Urgent attention needed',
  },
  medium: {
    label: 'Medium',
    badge: 'badge-warning',
    dot: 'status-dot-warning',
    chip: 'chip-warning',
    rank: 2,
    description: 'Attention needed soon',
  },
  low: {
    label: 'Low',
    badge: 'badge-info',
    dot: 'status-dot-info',
    chip: 'chip-default',
    rank: 3,
    description: 'Review when convenient',
  },
  info: {
    label: 'Info',
    badge: 'badge-neutral',
    dot: 'status-dot-neutral',
    chip: 'chip-default',
    rank: 4,
    description: 'Informational only',
  },
};

// =============================================================================
// EVENT TYPE
// Matches TrackerEvent.eventType from types.ts
// =============================================================================

export type EventType = 'compact' | 'escalation' | 'pane.status' | 'session.status';

export const EVENT_TYPE: Record<EventType, {
  label: string;
  badge: string;
  icon: string;
  description: string;
}> = {
  compact: {
    label: 'Compact',
    badge: 'badge-warning',
    icon: '⚡',
    description: 'Context was compacted/reset',
  },
  escalation: {
    label: 'Escalation',
    badge: 'badge-error',
    icon: '⚠️',
    description: 'Human attention required',
  },
  'pane.status': {
    label: 'Pane',
    badge: 'badge-info',
    icon: '📋',
    description: 'Pane status changed',
  },
  'session.status': {
    label: 'Session',
    badge: 'badge-info',
    icon: '🔄',
    description: 'Session status changed',
  },
};

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/**
 * Get connection status config with fallback to disconnected
 */
export function getConnectionStatus(status: string): typeof CONNECTION_STATUS[ConnectionStatus] {
  return CONNECTION_STATUS[status as ConnectionStatus] ?? CONNECTION_STATUS.disconnected;
}

/**
 * Get session status config with fallback to unknown
 */
export function getSessionStatus(status: string): typeof SESSION_STATUS[SessionStatus] {
  return SESSION_STATUS[status as SessionStatus] ?? SESSION_STATUS.unknown;
}

/**
 * Get pane status config with fallback to unknown
 */
export function getPaneStatus(status: string): typeof PANE_STATUS[PaneStatus] {
  return PANE_STATUS[status as PaneStatus] ?? PANE_STATUS.unknown;
}

/**
 * Get escalation severity config with fallback to info
 */
export function getEscalationSeverity(severity: string): typeof ESCALATION_SEVERITY[EscalationSeverity] {
  return ESCALATION_SEVERITY[severity as EscalationSeverity] ?? ESCALATION_SEVERITY.info;
}

/**
 * Get event type config with fallback to pane.status
 */
export function getEventType(type: string): typeof EVENT_TYPE[EventType] {
  return EVENT_TYPE[type as EventType] ?? EVENT_TYPE['pane.status'];
}

/**
 * Sort sessions by status (active first, then idle, then ended)
 */
export function sortBySessionStatus<T extends { status: string }>(items: T[]): T[] {
  return [...items].sort((a, b) => {
    const rankA = SESSION_STATUS[a.status as SessionStatus]?.rank ?? 4;
    const rankB = SESSION_STATUS[b.status as SessionStatus]?.rank ?? 4;
    return rankA - rankB;
  });
}

/**
 * Sort escalations by severity (critical first)
 */
export function sortByEscalationSeverity<T extends { severity: string }>(items: T[]): T[] {
  return [...items].sort((a, b) => {
    const rankA = ESCALATION_SEVERITY[a.severity as EscalationSeverity]?.rank ?? 4;
    const rankB = ESCALATION_SEVERITY[b.severity as EscalationSeverity]?.rank ?? 4;
    return rankA - rankB;
  });
}
