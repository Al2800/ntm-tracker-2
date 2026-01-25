# NTM Tracker Information Architecture

This document defines the navigation structure, visibility rules, and route hierarchy for NTM Tracker, based on the CodexMonitor-inspired UI patterns (see `docs/ui-patterns-codexmonitor.md`).

## Navigation Hierarchy

### Full Window Mode

```
┌────────────────────────────────────────────────────────────────────────┐
│  TOP COMMAND BAR                                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐
│  │ [Brand] │ Search (Ctrl+K) │ [Notifications] [Settings] [Status]   │
│  └─────────────────────────────────────────────────────────────────────┘
├──────────────────┬─────────────────────────────────────────────────────┤
│  SIDEBAR         │  MAIN CONTENT AREA                                  │
│  ┌──────────────┐│  ┌───────────────────────────────────────────────┐ │
│  │ Sessions Hub ││  │ FOCUS PANEL                                   │ │
│  │ ───────────  ││  │ Selected session details                      │ │
│  │ [Session 1]  ││  │ - Session info + status                       │ │
│  │ [Session 2]  ││  │ - Pane list with status indicators            │ │
│  │ [Session 3]  ││  │ - Output preview                              │ │
│  │ ...          ││  │ - Actions (attach, kill)                      │ │
│  │              ││  └───────────────────────────────────────────────┘ │
│  │ ───────────  ││  ┌───────────────────────────────────────────────┐ │
│  │ Quick Stats  ││  │ INSIGHTS PANEL                                │ │
│  │ - Active: 3  ││  │ Activity graph, escalations, timeline         │ │
│  │ - Alerts: 1  ││  └───────────────────────────────────────────────┘ │
│  └──────────────┘│                                                     │
└──────────────────┴─────────────────────────────────────────────────────┘
```

### Tray Popover Mode (Compact)

```
┌─────────────────────────────────┐
│ NTM Tracker         [●] 2 alerts│
├─────────────────────────────────┤
│ ● api:0              active     │
│ ● research           idle       │
│ ○ backend            ended      │
│ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
│ ⚠ auth timeout (click to view) │
├─────────────────────────────────┤
│ [Mute] [Settings] [Open App]    │
└─────────────────────────────────┘
```

## Panel Descriptions

### 1. Top Command Bar (Always Visible)

| Element | Purpose | Behavior |
|---------|---------|----------|
| Brand | Identity | Click opens dashboard |
| Search | Filter sessions/panes | Ctrl+K to focus, real-time filtering |
| Notifications | Toggle alerts | Badge shows unread count |
| Settings | Open settings | Opens /settings route |
| Connection Status | Show daemon state | Badge: connected/reconnecting/degraded |

### 2. Sidebar - Sessions Hub (Always Visible in Full Window)

| Element | Purpose | Behavior |
|---------|---------|----------|
| Session List | Browse all sessions | Sorted by status (active first), then name |
| Session Card | Quick status view | Status indicator, name, UID preview |
| Quick Actions | Per-session actions | Hover reveals: attach, kill, copy command |
| Quick Stats | Overview metrics | Active count, pending escalations |
| Filters | Narrow view | Status filter (all/active/idle/ended) |

**Sorting Options:**
- Status (default): active > idle > ended > unknown
- Name: alphabetical
- Activity: most recently active first

### 3. Focus Panel (Main Content)

| Element | Purpose | Behavior |
|---------|---------|----------|
| Session Header | Selected session info | Name, UID, status, created time |
| Pane List | Session's panes | Status badges, agent type, token count |
| Output Preview | Live pane output | Syntax-highlighted, scrollable |
| Actions Bar | Session commands | Attach, kill, copy attach command |

**Visibility:**
- Empty state when no session selected
- Populated when session clicked in sidebar

### 4. Insights Panel (Below Focus or Sidebar)

| Element | Purpose | Behavior |
|---------|---------|----------|
| Activity Graph | Time-series activity | 24h rolling, events/hour |
| Escalation Panel | Pending alerts | Dismissable, click to focus |
| Timeline | Recent events | Compact event list with timestamps |

## Route Structure

| Route | Purpose | Mode |
|-------|---------|------|
| `/` | Main dashboard | Full window |
| `/?view=compact` | Tray popover view | Compact (320x400px) |
| `/settings` | App configuration | Full window |
| `/wizard` | First-run setup | Full window |

### Route Parameters

| Param | Values | Purpose |
|-------|--------|---------|
| `view` | `compact` | Tray popover layout |
| `focusSearch` | `1` | Auto-focus search on load |
| `session` | `<uid>` | Pre-select session |

## Visibility Rules

### Always Visible (Both Modes)

- Connection status indicator
- Session list (compact in tray mode)
- Escalation alert badge

### Full Window Only

- Sidebar sessions hub
- Activity graph
- Timeline panel
- Output preview (large)

### Tray Popover Only

- Compact session cards (top 4)
- Single-line escalation summary
- Quick action buttons (mute, settings, open)

### Contextual (On Demand)

- Pane list (when session selected)
- Output preview (when pane selected)
- Session actions (on hover/focus)

### Deep Drill-Down

- Full escalation details (click escalation badge)
- Complete output history (click output preview)
- Session history (click timeline entry)

## Tray vs Full Window Differences

| Feature | Full Window | Tray Popover |
|---------|-------------|--------------|
| **Session List** | All sessions, expandable | Top 4, compact cards |
| **Session Details** | Full pane list + output | Status only |
| **Activity Graph** | Full 24h visualization | Hidden |
| **Escalations** | List with details | Count badge + 1 summary |
| **Timeline** | Full event list | Hidden |
| **Search** | Full search bar | Hidden (limited space) |
| **Window Size** | Resizable (min 800x600) | Fixed (320x400) |
| **Actions** | Full action bar | Quick action row |

## Component Mapping

```
routes/+page.svelte
├── Header (brand + status)
├── CommandBar
│   ├── SearchInput
│   ├── NotificationToggle
│   └── SettingsButton
├── [Full Window Only]
│   ├── Sidebar
│   │   ├── SessionsHub
│   │   │   ├── FilterBar
│   │   │   └── SessionList > SessionCard
│   │   └── QuickStats
│   └── MainContent
│       ├── FocusPanel
│       │   ├── SessionHeader
│       │   ├── PaneList > PaneCard
│       │   ├── OutputPreview
│       │   └── ActionsBar
│       └── InsightsPanel
│           ├── ActivityGraph
│           ├── EscalationPanel
│           └── TimelinePanel
├── [Tray Popover Only]
│   ├── CompactSessionList
│   ├── EscalationSummary
│   └── QuickActionRow

routes/settings/+page.svelte
├── SettingsHeader
├── AutostartToggle
├── NotificationSettings
├── QuietHoursConfig
├── TransportConfig
└── DiagnosticsExport

routes/wizard/+page.svelte
├── WizardHeader
├── DistroSelector
├── DaemonInstaller
└── ConfigValidator
```

## Keyboard Navigation

| Key | Action |
|-----|--------|
| `Ctrl+K` | Focus search |
| `Esc` | Clear selection / close popover |
| `↑/↓` | Navigate session list |
| `Enter` | Select session |
| `Ctrl+.` | Toggle notification mute |

## Responsive Breakpoints

| Breakpoint | Behavior |
|------------|----------|
| `< 640px` | Stack panels vertically |
| `640-1024px` | Narrow sidebar, main content |
| `> 1024px` | Full sidebar + focus + insights |

## Implementation Notes

1. **Sidebar persistence**: Store collapsed/expanded state in localStorage
2. **Panel resizing**: Allow drag-resize between sidebar and main content
3. **Route transitions**: Smooth fade/slide between routes
4. **Session selection**: Store last selected session, restore on reload
5. **Tray window**: Separate Tauri window with fixed dimensions
