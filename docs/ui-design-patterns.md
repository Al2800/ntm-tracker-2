# UI Design Patterns: CodexMonitor Inspiration for NTM Tracker

> Design note for bd-212.1.1 — Capturing CodexMonitor UI patterns and NTM UX goals

## Reference

- **CodexMonitor**: https://github.com/Dimillian/CodexMonitor
- **Website**: https://www.codexmonitor.app/

CodexMonitor is a macOS Tauri app for orchestrating Codex agents. Its "command center" approach and visual polish serve as inspiration for elevating NTM Tracker's UI.

---

## CodexMonitor Patterns

### Layout Architecture

| Pattern | Description |
|---------|-------------|
| **Sidebar-driven navigation** | Left sidebar for workspace/project management with collapsible groups |
| **Resizable multi-panel** | Main area + right context panels + bottom dock, all resizable with persisted sizes |
| **Home dashboard** | Quick-access recent activity and workspace status |
| **Focus panel** | Selected item expands into detailed view |

### Visual Polish ("Stripe-grade" refinements)

- **Persisted panel sizes** — Layout remembers user preferences
- **macOS overlay title bar** — Native-feeling window chrome
- **Usage meter in sidebar** — Credit/rate-limit visualization
- **Toast-driven updates** — Non-intrusive feedback for background operations
- **Sound notifications** — Auditory alerts for important events
- **Reduced transparency toggle** — Accessibility option

### Component Patterns

- **Thread management** — Pin/rename/archive/copy with draft preservation
- **Status indicators** — Running, unread, active states clearly shown
- **Autocomplete** — Skills ($), prompts (/), file paths (@)
- **Quick actions** — Context menu + keyboard shortcuts

---

## Patterns That Map to NTM Tracker

| CodexMonitor | NTM Tracker Equivalent | Priority |
|--------------|------------------------|----------|
| Workspace sidebar | Session list sidebar | High |
| Home dashboard | OverviewCards + quick status | High |
| Thread focus view | Session focus panel (panes + output) | High |
| Running/unread indicators | Active/idle/escalation badges | High |
| Resizable panels | Session list + focus panel split | Medium |
| Usage meter | Token estimate / active hours display | Medium |
| Toast updates | Escalation notifications | Medium |
| Quick actions | Session context menu (kill pane, clear focus) | Medium |
| Persisted layout | Remember panel widths, collapsed states | Low |

---

## Patterns That Do NOT Translate

| CodexMonitor Feature | Why Not Applicable |
|---------------------|-------------------|
| Multi-project management | NTM Tracker monitors a single set of sessions from one daemon |
| Agent chat/conversation view | NTM monitors sessions, doesn't interact conversationally |
| Worktree/clone management | Git integration out of scope |
| Prompt library | No prompt authoring workflow |
| Composer with queuing | No message composition |
| Git diff rendering | Session monitoring, not code review |
| Model/reasoning effort controls | Backend protocol fixed |

---

## NTM-Specific UX Requirements

### Core Workflows

1. **Tray-first experience**
   - Compact popover (/?view=compact) must be useful standalone
   - Show: top 4 sessions, pending escalations, connection status
   - Quick actions: open dashboard, mute notifications, settings

2. **Session monitoring at a glance**
   - Session list with status badges (active/idle/ended)
   - Pane count per session
   - Last activity timestamp

3. **Escalation visibility**
   - Prominent alert count in overview
   - Dedicated escalation panel with dismiss/acknowledge
   - Notification support (toast + sound during non-quiet hours)

4. **Drill-down: Session → Pane → Output**
   - Click session to expand pane list
   - Select pane to preview live output
   - Output preview with redaction for sensitive data

5. **Compact detection events**
   - Today's compact count in overview
   - Timeline of recent compacts
   - Per-session compact history

### Layout Proposal

```
+--------------------------------------------------+
|  NTM Tracker                    [Connection] [?] |
+--------------------------------------------------+
| Sidebar          |  Focus Panel                  |
| +-------------+  |  +-------------------------+  |
| | Sessions    |  |  | Selected Session        |  |
| | - api:0  *  |  |  | Panes: 3                |  |
| | - research  |  |  | +---+ +---+ +---+       |  |
| | - test      |  |  | |P1 | |P2 | |P3 |       |  |
| +-------------+  |  | +---+ +---+ +---+       |  |
| | Quick Stats |  |  +-------------------------+  |
| | 3 sessions  |  |  | Output Preview          |  |
| | 2 escalates |  |  | ...                     |  |
| +-------------+  |  +-------------------------+  |
+------------------+-------------------------------+
|  Activity Timeline                               |
+--------------------------------------------------+
```

### Tray Popover

```
+------------------------+
| NTM Tracker   [Open]   |
+------------------------+
| api:0         active   |
| research      idle     |
| test          ended    |
+------------------------+
| 2 escalations pending  |
+------------------------+
| Quiet: 22:00-08:00     |
+------------------------+
```

---

## Design Tokens Direction

### Color Palette

Keep the existing dark slate foundation:
- **Background**: slate-950, slate-900/60
- **Borders**: slate-800/80
- **Text**: slate-100, slate-300, slate-400, slate-500
- **Accents**:
  - Emerald for connected/success
  - Amber for warning/reconnecting
  - Rose for error/escalation
  - Sky for info/connecting

### Typography

- **Uppercase tracking** for labels: `text-xs uppercase tracking-[0.2em]`
- **Semibold white** for values/headings
- **Slate-500** for supporting text

### Component Patterns

- **Cards**: `rounded-2xl border border-slate-800/80 bg-slate-900/60 p-4`
- **Badges**: `rounded-full px-3 py-1 text-xs` with ring accents
- **Inputs**: `rounded-lg border border-slate-700/80 bg-slate-950`

---

## Success Criteria

1. Clear, elegant navigation (sidebar + focus panel)
2. Session + pane info readable at a glance
3. Tray popover useful standalone
4. Visual consistency across all components
5. Accessibility: keyboard navigation, focus states, contrast ratios

---

## Next Steps

This design note informs:
- **bd-212.1.2**: Define IA + navigation map
- **bd-212.1.3**: Tray popover UX spec
- **bd-212.2.1**: Define design tokens + typography scale
