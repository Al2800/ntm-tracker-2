# Tray Popover UX Specification

## Overview

The tray popover provides a compact, glanceable view of NTM session status without opening the full dashboard. It appears on left-click of the system tray icon and follows native Windows popover conventions.

## Window Configuration

| Property | Value | Rationale |
|----------|-------|-----------|
| Size | 380x520 px | Fits 6-8 sessions + stats without scroll in typical cases |
| Position | Near tray icon | Positioned at click coordinates, offset to appear above/left of click |
| Decorations | None | Clean, native popover feel |
| Always on top | Yes | Stays visible while interacting |
| Skip taskbar | Yes | Doesn't clutter taskbar |
| Resizable | No | Fixed compact layout |
| Shadow | Yes | Visual separation from desktop |

## Behavior

### Opening
- **Trigger**: Left-click on tray icon
- **Animation**: Instant show (no fade), positioned near click
- **Focus**: Popover receives focus immediately
- **State**: Loads `/?view=compact` route

### Closing
- **Blur**: Closes when popover loses focus (click outside)
- **Toggle**: Left-click on tray icon again closes popover
- **Escape**: (Future) Close on Escape key
- **Navigation**: Closes when navigating to full dashboard

### Interactions
- Clicks inside popover do not close it
- Double-click on tray icon opens full dashboard (bypasses popover)
- Right-click on tray icon shows context menu (unchanged)

## Content Layout

### Header (fixed)
```
[ NTM Tracker (text)  |  [Connection badge] ]
```
- Minimal branding
- Connection status pill (connected/disconnected/etc.)

### Stats Grid
```
┌─────────────┬─────────────┬─────────────┐
│   Sessions  │    Active   │    Alerts   │
│     (N)     │     (N)     │     (N)     │
└─────────────┴─────────────┴─────────────┘
```
- Bold numbers with small labels
- Green for active, amber for alerts

### Session List (scrollable, max 8 visible)
```
┌─────────────────────────────────────────┐
│ session-name                   [status] │
│ session-name-2                 [status] │
│ ...                                     │
└─────────────────────────────────────────┘
```
- Session name truncated if too long
- Status badge (active/idle/ended) color-coded
- "+N more" indicator if >8 sessions

### Pending Alerts (conditional)
```
┌─────────────────────────────────────────┐
│ PENDING ALERTS                          │
│ [Alert message truncated...]            │
│ [Alert message 2...]                    │
│ +N more                                 │
└─────────────────────────────────────────┘
```
- Amber border/background
- Only shown if alerts > 0
- Max 3 shown with overflow indicator

### Quick Actions
```
[ Mute/Unmute ] [ Settings ]
```
- Two equal-width buttons
- Mute toggles notification state
- Settings navigates to settings page

## Data Points Displayed

| Data | Source | Update Frequency |
|------|--------|------------------|
| Session count | snapshot.get | 2s |
| Active pane count | snapshot.get | 2s |
| Alert count | events (escalations) | 2s |
| Session list (top 8) | snapshot.get | 2s |
| Session status | snapshot.get | 2s |
| Connection state | connection store | Real-time |

## Platform Constraints (Windows)

### Tauri 2.0 Considerations
- `alwaysOnTop: true` keeps popover above other windows
- `skipTaskbar: true` prevents taskbar entry
- `decorations: false` removes title bar
- `focus: true` ensures immediate keyboard focus
- Blur event listening via `tauri://blur` event

### Windows Tray Behavior
- Tray icon receives `TrayIconEvent::Click` with position
- Position is in screen coordinates, used for popover placement
- Left-click distinguished from double-click (200ms threshold in Tauri)

### Shadow Support
- `shadow: true` in window config for drop shadow
- Provides visual separation from desktop

## Future Enhancements

1. **Keyboard shortcuts**: Escape to close, arrow keys to navigate sessions
2. **Session quick actions**: Attach to session, view output preview
3. **Pin option**: Keep popover open even on blur
4. **Theme sync**: Match system light/dark mode
5. **Resize handle**: Allow user-adjustable size (persisted)

## Related Documents

- [UI Design Patterns](ui-design-patterns.md)
- [CodexMonitor Patterns](ui-patterns-codexmonitor.md)
- [Information Architecture](information-architecture.md)
