# CodexMonitor UI Patterns for NTM Tracker

This document captures the UI/UX patterns from [CodexMonitor](https://github.com/Dimillian/CodexMonitor) that inform the NTM Tracker redesign, identifies what translates to our use case, and notes what does not apply.

## Reference

- **CodexMonitor**: macOS/Linux Tauri app for orchestrating Codex agents across workspaces
- **Tech Stack**: Tauri + React (we use Tauri + Svelte 5)
- **Design Goal**: "Stripe-level polish" - clean, modern command-center aesthetic

## Patterns We're Adopting

### 1. Multi-Panel Layout

**CodexMonitor Pattern:**
- Persistent sidebar for workspace/project navigation
- Main content area with resizable panels
- Right panel for context-dependent information
- Terminal dock (bottom) for background commands

**NTM Tracker Translation:**
- **Sidebar**: Session list with status indicators, filters, quick actions
- **Main Panel**: Focus panel showing selected session/pane details
- **Right Panel**: Activity graph, escalations, timeline
- **Optional Bottom Dock**: Output preview (already exists, can be enhanced)

### 2. Command Bar / Top Navigation

**CodexMonitor Pattern:**
- Top command bar with model picker, usage meter, quick actions
- Search with autocomplete (skills, prompts, files)

**NTM Tracker Translation:**
- **Top Bar**: Connection status badge, search (Ctrl+K), notification toggle, settings
- **Search**: Filter sessions by name/UID (already implemented)
- **Usage Meter**: Could show active panes, pending escalations count

### 3. Sessions Hub

**CodexMonitor Pattern:**
- Projects hub with grouping, sorting, recent activity
- Visual indicators for unread/running state
- Quick jump to recent threads

**NTM Tracker Translation:**
- **Sessions List**: Already have status badges (active/idle/ended)
- **Add**: Sorting options (status, name, activity), grouping by status
- **Add**: Quick action buttons (attach, kill, view output)
- **Add**: Unread escalation indicators per session

### 4. Focus Panel

**CodexMonitor Pattern:**
- Conversation view with thread management
- Context-aware right panel

**NTM Tracker Translation:**
- **Session Focus**: Currently shows pane list + output preview
- **Enhance**: Add session actions (kill, attach command copy)
- **Enhance**: Richer pane cards with agent type, token count, last activity

### 5. Status Visualization

**CodexMonitor Pattern:**
- Context usage ring (credits visualization)
- Rate limit indicators in sidebar

**NTM Tracker Translation:**
- **Overview Cards**: Already show session/pane/escalation counts
- **Add**: Token usage summary (we have token_estimator in daemon)
- **Add**: Activity sparkline per session

### 6. Tray Popover

**CodexMonitor Pattern:**
- Compact, glanceable overlay
- Quick actions without opening full app

**NTM Tracker Translation:**
- **Compact Mode**: Already have `?view=compact` route
- **Enhance**: Make it tray-window sized (narrow, fixed height)
- **Content**: Top 4 sessions, pending escalations count, quick mute toggle
- **Actions**: Click session to attach, dismiss escalations

### 7. Visual Polish

**CodexMonitor Pattern:**
- macOS vibrancy/overlay title bar
- Toast notifications for updates
- Resizable panels with persisted sizes
- Syntax-highlighted diffs

**NTM Tracker Translation:**
- **Title Bar**: Tauri supports custom title bar on Windows
- **Toasts**: Add toast component for real-time events
- **Panel Sizing**: Persist user preferences to localStorage
- **Output Preview**: Add syntax highlighting (already partially there)

## Patterns That Do NOT Translate

### 1. Multi-Project Management
CodexMonitor manages multiple workspaces/projects. NTM Tracker monitors a single NTM instance with multiple sessions. No project picker needed.

### 2. Agent Chat/Conversation History
CodexMonitor shows LLM conversation threads. NTM Tracker focuses on session monitoring, not conversation replay. We show escalations and events, not full transcripts.

### 3. Git Diff Viewer / File Tree
CodexMonitor integrates file browsing and diffs. NTM Tracker is monitoring-focused; we show output preview, not codebase navigation.

### 4. Worktree/Clone Management
CodexMonitor spawns isolated agent instances. NTM Tracker observes existing tmux sessions; we don't create them.

### 5. Compose/Autocomplete
CodexMonitor has rich message composition with skill/prompt autocomplete. NTM Tracker is read-only monitoring; no message sending.

### 6. Remote Backend Mode
CodexMonitor can run Codex on another machine. NTM Tracker is inherently local (WSL2 on same machine).

## Visual Direction

### Color Palette (current, refine as needed)
- **Background**: `slate-950` (deep blue-gray)
- **Cards**: `slate-900/60` with `slate-800/80` borders
- **Text**: `slate-100` primary, `slate-400` secondary
- **Accents**:
  - Emerald for connected/success
  - Amber for warnings/reconnecting
  - Rose for errors/degraded
  - Sky for loading/connecting

### Typography
- **Headers**: Uppercase tracking-wide (`tracking-[0.2em]`)
- **Body**: Clean sans-serif (system fonts via Tailwind defaults)
- **Monospace**: For UIDs, output preview, code

### Spacing
- Consistent `rounded-2xl` for cards
- `p-5` or `p-6` internal padding
- `gap-6` between sections

## Component Hierarchy

```
Dashboard (main)
├── Header
│   ├── Brand + tagline
│   └── Connection badge
├── Command Bar
│   ├── Search input (Ctrl+K)
│   ├── Settings button
│   ├── Notifications toggle
│   └── Wizard button
├── Overview Cards (stats row)
├── Main Grid (2-column on lg+)
│   ├── Left Column
│   │   ├── Sessions Panel (with SessionList)
│   │   └── Activity Graph
│   └── Right Column
│       ├── Tray Preview (mini session list)
│       ├── Session Focus (when selected)
│       │   ├── Session info
│       │   ├── PaneList
│       │   └── OutputPreview
│       ├── Escalation Panel
│       └── Timeline Panel
```

## Tray Popover (Compact Mode)

When opened via tray click:
```
┌─────────────────────────────┐
│ NTM Tracker      [●] 2 alerts│
├─────────────────────────────┤
│ ● api:0          active      │
│ ● research       idle        │
│ ○ backend        ended       │
├─────────────────────────────┤
│ ⚠ Escalation: auth timeout  │
├─────────────────────────────┤
│ [Mute] [Settings] [Dashboard]│
└─────────────────────────────┘
```

- Fixed size: ~300x400px
- Close on blur
- Sessions link to attach command
- Escalation click opens full dashboard

## Next Steps

With this pattern documentation in place, downstream tasks can proceed:

1. **bd-212.2.1**: Define design tokens + typography scale based on this palette
2. **bd-212.1.2**: Define IA + navigation map using component hierarchy above
3. **bd-212.1.3**: Tray popover UX spec using the compact mode wireframe

## References

- [CodexMonitor GitHub](https://github.com/Dimillian/CodexMonitor)
- [CodexMonitor Website](https://www.codexmonitor.app/)
