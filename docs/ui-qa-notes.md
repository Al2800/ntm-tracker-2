# UI/UX QA Notes

Final visual QA pass for NTM Tracker's CodexMonitor-inspired redesign.

## Component Checklist

### Layout Shell
- [x] DashboardLayout provides correct slots for command-bar, sidebar, focus, insights
- [x] Compact/tray mode uses single-column layout
- [x] Skip link for keyboard navigation
- [x] Proper ARIA landmarks (navigation, main, complementary)
- [x] Scrollbar styling consistent across panels

### Command Bar
- [x] Brand link with focus ring
- [x] Search input with Ctrl+K shortcut
- [x] Notification toggle with pending count badge
- [x] Settings button with aria-label
- [x] Connection status badge with aria-live

### Sessions Hub (Sidebar)
- [x] Filter chips with memoized counts
- [x] Sort dropdown with label
- [x] Session list with render limit for performance
- [x] Empty state for filtered/unfiltered views
- [x] SessionItem with status dot, name, UID, pane counts
- [x] Hover/focus shows quick actions
- [x] CSS containment for scroll performance

### Focus Panel
- [x] Session detail with expanded info
- [x] Pane list with output preview
- [x] ErrorBanner for connection issues
- [x] LoadingSkeleton during data fetch

### Insights Panel
- [x] Collapsible sections with aria-expanded
- [x] ActivityGraph with hour-by-hour bars
- [x] EscalationPanel with severity badges
- [x] TimelinePanel with chronological events
- [x] EmptyState for each section

### State Components
- [x] EmptyState with 6 icon variants (sessions, escalations, timeline, search, output, generic)
- [x] LoadingSkeleton with 4 variants (card, list-item, text, chart)
- [x] ErrorBanner with 3 severity levels (error, warning, info)

## Typography & Spacing

### Type Scale
- `text-2xs` (10px): Badges, timestamps, chip counts
- `text-xs` (12px): Labels, captions, secondary text
- `text-sm` (14px): Body text, list items
- `text-base` (16px): Primary content

### Spacing Scale
- `p-card-sm` / `p-3`: Compact cards
- `p-card-md` / `p-4`: Standard cards
- `p-card-lg` / `p-6`: Large cards
- `gap-1.5`: Tight list spacing
- `gap-2` to `gap-3`: Standard spacing
- `gap-4`: Section spacing

## Status Treatments

### Connection States
| State | Badge | Color |
|-------|-------|-------|
| Connected | `badge-success` | Green |
| Connecting | `badge-info` | Blue |
| Reconnecting | `badge-warning` | Amber |
| Degraded | `badge-error` | Red |
| Disconnected | `badge-neutral` | Gray |

### Session States
| State | Dot Color |
|-------|-----------|
| Active | `bg-status-success` |
| Idle | `bg-status-neutral` |
| Waiting | `bg-status-warning` |
| Ended | `bg-status-error` |

### Escalation Severity
| Level | Badge | Card |
|-------|-------|------|
| High | `badge-error` | `card-critical` |
| Medium | `badge-warning` | Standard |
| Low | `badge-info` | Standard |

## Accessibility

### Keyboard Navigation
- Ctrl+K: Focus search
- Tab: Navigate between sections
- Arrow keys: Navigate lists
- Enter/Space: Select/toggle
- Escape: Close popovers

### ARIA Support
- All interactive elements have aria-labels
- Collapsible sections use aria-expanded/aria-controls
- Status updates use aria-live
- Lists use role="list"/role="listitem"

### Focus Management
- `.focus-ring` class for keyboard-visible focus
- Skip link to main content
- No focus traps in dialogs

## Motion

### Transitions
- `transition-colors`: Hover color changes
- `transition-all`: Card hover effects
- `animate-fade-in`: Content reveals
- `animate-pulse-slow`: Loading states

### Reduced Motion
- All animations respect `prefers-reduced-motion`
- Static alternatives provided

## Performance

### Optimizations Applied
- Memoized filter counts in SessionsHub
- Render limit (50 items) with "show more"
- CSS containment for scroll performance
- will-change hints for scroll containers

## Follow-up Recommendations

1. **Screenshot Automation**: Add Playwright tests to capture screenshots on each build
2. **Theme Support**: Consider light mode or system preference detection
3. **Responsive Breakpoints**: Test on narrower viewports (tablet)
4. **i18n**: Prepare for localization if needed
