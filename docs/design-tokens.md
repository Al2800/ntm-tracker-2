# NTM Tracker Design Tokens

This document describes the design token system used in NTM Tracker. Tokens are defined in `app/tailwind.config.cjs` with component utilities in `app/src/app.css`.

## Color Tokens

### Surface (Backgrounds)

| Token | Class | Use Case |
|-------|-------|----------|
| `surface-base` | `bg-surface-base` | Page background, deepest layer |
| `surface-raised` | `bg-surface-raised` | Cards, panels |
| `surface-elevated` | `bg-surface-elevated` | Modals, popovers |
| `surface-overlay` | `bg-surface-overlay` | Overlays, backdrops |

### Text Hierarchy

| Token | Class | Use Case |
|-------|-------|----------|
| `text-primary` | `text-text-primary` | Main content, headings |
| `text-secondary` | `text-text-secondary` | Supporting text |
| `text-muted` | `text-text-muted` | Labels, captions |
| `text-subtle` | `text-text-subtle` | Hints, placeholders |
| `text-inverse` | `text-text-inverse` | Text on light/accent backgrounds |

### Borders

| Token | Class | Use Case |
|-------|-------|----------|
| `border` | `border-border` | Primary borders |
| `border-subtle` | `border-border-subtle` | Subtle dividers |
| `border-strong` | `border-border-strong` | Emphasized borders |
| `border-focus` | `ring-border-focus` | Focus rings |

### Status Colors

Each status has four variants: DEFAULT, muted (background), ring, and text.

| Status | Meaning | Badge Class |
|--------|---------|-------------|
| `success` | Connected, active | `.badge-success` |
| `warning` | Reconnecting, idle | `.badge-warning` |
| `error` | Degraded, ended | `.badge-error` |
| `info` | Connecting, loading | `.badge-info` |
| `neutral` | Disconnected, unknown | `.badge-neutral` |

**Usage:**
```html
<!-- Direct token usage -->
<span class="bg-status-success-muted text-status-success-text ring-1 ring-status-success-ring">
  Connected
</span>

<!-- Component class (preferred) -->
<span class="badge badge-success">Connected</span>
```

### Accent

| Token | Class | Use Case |
|-------|-------|----------|
| `accent` | `bg-accent` / `text-accent` | Primary interactive elements |
| `accent-hover` | `hover:bg-accent-hover` | Hover state |
| `accent-muted` | `bg-accent-muted` | Subtle accent backgrounds |

## Typography

### Letter Spacing (Headers)

| Token | Class | Value | Use Case |
|-------|-------|-------|----------|
| `label` | `tracking-label` | 0.2em | Standard labels |
| `label-tight` | `tracking-label-tight` | 0.15em | Compact labels |
| `label-wide` | `tracking-label-wide` | 0.3em | Emphasized labels |
| `label-widest` | `tracking-label-widest` | 0.4em | Hero/branding |

### Font Sizes

| Token | Class | Size | Use Case |
|-------|-------|------|----------|
| `2xs` | `text-2xs` | 10px | Tray compact, dense views |
| `xs` | `text-xs` | 12px | Labels, badges |
| `sm` | `text-sm` | 14px | Body text, buttons |
| `base` | `text-base` | 16px | Primary content |
| `xl` | `text-xl` | 20px | Section headers |
| `4xl` | `text-4xl` | 36px | Page titles |

## Spacing

### Card Padding

| Token | Class | Value | Use Case |
|-------|-------|-------|----------|
| `card-sm` | `p-card-sm` | 16px | Compact cards |
| `card-md` | `p-card-md` | 20px | Standard cards |
| `card-lg` | `p-card-lg` | 24px | Feature cards |
| `tray` | `p-tray` | 6px | Tray ultra-compact |

### Border Radius

| Token | Class | Value | Use Case |
|-------|-------|-------|----------|
| `card` | `rounded-card` | 16px | Standard cards |
| `card-lg` | `rounded-card-lg` | 20px | Feature sections |

## Elevation (Shadows)

| Token | Class | Use Case |
|-------|-------|----------|
| `elevation-1` | `shadow-elevation-1` | Subtle lift |
| `elevation-2` | `shadow-elevation-2` | Cards, buttons |
| `elevation-3` | `shadow-elevation-3` | Modals, dropdowns |
| `glow-accent` | `shadow-glow-accent` | Focus highlight |
| `glow-success` | `shadow-glow-success` | Success highlight |

## Component Classes

Pre-built component classes in `app.css`:

### Cards
```html
<div class="card">Standard card</div>
<div class="card-lg">Large feature card</div>
<div class="card-compact">Compact card</div>
```

### Labels
```html
<span class="label">Section Label</span>
<span class="label-sm">Small Label</span>
<span class="label-lg">Large Label</span>
```

### Badges
```html
<span class="badge badge-success">Connected</span>
<span class="badge badge-warning">Reconnecting</span>
<span class="badge badge-error">Degraded</span>
<span class="badge badge-info">Connecting</span>
<span class="badge badge-neutral">Disconnected</span>
```

### Buttons
```html
<button class="btn btn-primary">Primary Action</button>
<button class="btn btn-secondary">Secondary</button>
<button class="btn btn-ghost">Ghost</button>
```

### Inputs
```html
<input class="input" placeholder="Search..." />
```

### Tray Items
```html
<div class="tray-item">Standard tray row</div>
<div class="tray-item-compact">Compact tray row</div>
```

## Animation

| Token | Class | Use Case |
|-------|-------|----------|
| `pulse-slow` | `animate-pulse-slow` | Slow pulsing indicator |
| `fade-in` | `animate-fade-in` | Fade entrance |
| `slide-up` | `animate-slide-up` | Slide up entrance |

## Background Gradients

```html
<!-- Hero radial gradient -->
<div class="bg-gradient-hero"></div>

<!-- Custom radial with stops -->
<div class="bg-gradient-radial from-accent/20 via-surface-base to-transparent"></div>
```

## Migration Guide

When updating existing components to use tokens:

| Old Pattern | New Pattern |
|-------------|-------------|
| `bg-slate-950` | `bg-surface-base` |
| `bg-slate-900/60` | `bg-surface-raised` |
| `border-slate-800/80` | `border-border` |
| `text-slate-100` | `text-text-primary` |
| `text-slate-400` | `text-text-muted` |
| `text-emerald-200` | `text-status-success-text` |
| `rounded-2xl` | `rounded-card-lg` |
| `p-5` | `p-card-md` |
