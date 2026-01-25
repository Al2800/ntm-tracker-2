/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      // ──────────────────────────────────────────────────────────────
      // SEMANTIC COLOR TOKENS
      // See docs/ui-patterns-codexmonitor.md for design rationale
      // ──────────────────────────────────────────────────────────────
      colors: {
        // Surface / background layers
        surface: {
          base: 'rgb(2 6 23)', // slate-950 equivalent - deepest background
          raised: 'rgb(15 23 42 / 0.6)', // slate-900/60 - card backgrounds
          elevated: 'rgb(15 23 42 / 0.8)', // slate-900/80 - modals, popovers
          overlay: 'rgb(15 23 42 / 0.95)', // near-opaque overlay
        },
        // Border colors
        border: {
          DEFAULT: 'rgb(30 41 59 / 0.8)', // slate-800/80 - primary borders
          subtle: 'rgb(51 65 85 / 0.6)', // slate-700/60 - subtle dividers
          strong: 'rgb(51 65 85)', // slate-700 - emphasized borders
          focus: 'rgb(56 189 248 / 0.5)', // sky-400/50 - focus rings
        },
        // Text hierarchy
        text: {
          primary: 'rgb(241 245 249)', // slate-100 - main content
          secondary: 'rgb(203 213 225)', // slate-300 - supporting text
          muted: 'rgb(148 163 184)', // slate-400 - labels, captions
          subtle: 'rgb(100 116 139)', // slate-500 - hints, placeholders
          inverse: 'rgb(15 23 42)', // slate-900 - text on light backgrounds
        },
        // Status indicators
        status: {
          // Connected / Success
          success: {
            DEFAULT: 'rgb(16 185 129)', // emerald-500
            muted: 'rgb(16 185 129 / 0.15)', // badge background
            ring: 'rgb(52 211 153 / 0.4)', // emerald-400/40
            text: 'rgb(167 243 208)', // emerald-200
          },
          // Warning / Reconnecting
          warning: {
            DEFAULT: 'rgb(245 158 11)', // amber-500
            muted: 'rgb(245 158 11 / 0.15)',
            ring: 'rgb(251 191 36 / 0.4)', // amber-400/40
            text: 'rgb(253 230 138)', // amber-200
          },
          // Error / Degraded
          error: {
            DEFAULT: 'rgb(244 63 94)', // rose-500
            muted: 'rgb(244 63 94 / 0.15)',
            ring: 'rgb(251 113 133 / 0.4)', // rose-400/40
            text: 'rgb(254 205 211)', // rose-200
          },
          // Info / Connecting
          info: {
            DEFAULT: 'rgb(14 165 233)', // sky-500
            muted: 'rgb(14 165 233 / 0.15)',
            ring: 'rgb(56 189 248 / 0.4)', // sky-400/40
            text: 'rgb(186 230 253)', // sky-200
          },
          // Neutral / Disconnected
          neutral: {
            DEFAULT: 'rgb(100 116 139)', // slate-500
            muted: 'rgb(100 116 139 / 0.15)',
            ring: 'rgb(100 116 139 / 0.4)',
            text: 'rgb(203 213 225)', // slate-300
          },
        },
        // Accent for interactive elements
        accent: {
          DEFAULT: 'rgb(56 189 248)', // sky-400 - primary accent
          hover: 'rgb(14 165 233)', // sky-500 - hover state
          muted: 'rgb(56 189 248 / 0.2)', // subtle accent backgrounds
        },
      },

      // ──────────────────────────────────────────────────────────────
      // TYPOGRAPHY SCALE
      // ──────────────────────────────────────────────────────────────
      fontSize: {
        // Dense sizes for tray/compact views
        '2xs': ['0.625rem', { lineHeight: '0.875rem' }], // 10px
        // Standard scale preserved, add semantic aliases via @apply
      },
      letterSpacing: {
        // Header tracking styles (uppercase labels)
        label: '0.2em',
        'label-tight': '0.15em',
        'label-wide': '0.3em',
        'label-widest': '0.4em',
      },

      // ──────────────────────────────────────────────────────────────
      // SPACING & LAYOUT
      // ──────────────────────────────────────────────────────────────
      spacing: {
        // Card internal padding
        'card-sm': '1rem', // 16px - compact cards
        'card-md': '1.25rem', // 20px - standard cards
        'card-lg': '1.5rem', // 24px - feature cards
        // Tray-specific compact spacing
        'tray': '0.375rem', // 6px - ultra-compact
      },
      borderRadius: {
        // Card radii
        'card': '1rem', // 16px - standard cards
        'card-lg': '1.25rem', // 20px - feature sections
      },

      // ──────────────────────────────────────────────────────────────
      // BOX SHADOW (Elevation)
      // ──────────────────────────────────────────────────────────────
      boxShadow: {
        // Subtle elevation for raised surfaces
        'elevation-1': '0 1px 2px 0 rgb(0 0 0 / 0.3)',
        'elevation-2': '0 2px 4px 0 rgb(0 0 0 / 0.3), 0 1px 2px -1px rgb(0 0 0 / 0.3)',
        'elevation-3': '0 4px 6px -1px rgb(0 0 0 / 0.3), 0 2px 4px -2px rgb(0 0 0 / 0.3)',
        // Glow effects for focus/hover
        'glow-accent': '0 0 12px 2px rgb(56 189 248 / 0.25)',
        'glow-success': '0 0 12px 2px rgb(16 185 129 / 0.25)',
        'glow-warning': '0 0 12px 2px rgb(245 158 11 / 0.25)',
        'glow-error': '0 0 12px 2px rgb(244 63 94 / 0.25)',
      },

      // ──────────────────────────────────────────────────────────────
      // ANIMATION
      // ──────────────────────────────────────────────────────────────
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'fade-in': 'fadeIn 0.2s ease-out',
        'fade-out': 'fadeOut 0.2s ease-out',
        'slide-up': 'slideUp 0.2s ease-out',
        'slide-down': 'slideDown 0.2s ease-out',
        'scale-in': 'scaleIn 0.15s ease-out',
        'lift': 'lift 0.2s ease-out forwards',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        fadeOut: {
          '0%': { opacity: '1' },
          '100%': { opacity: '0' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(4px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        slideDown: {
          '0%': { opacity: '0', transform: 'translateY(-4px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        scaleIn: {
          '0%': { opacity: '0', transform: 'scale(0.95)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
        lift: {
          '0%': { transform: 'translateY(0)' },
          '100%': { transform: 'translateY(-2px)' },
        },
      },

      // ──────────────────────────────────────────────────────────────
      // BACKGROUND IMAGES
      // ──────────────────────────────────────────────────────────────
      backgroundImage: {
        // Radial gradient for hero/header areas
        'gradient-radial': 'radial-gradient(circle at top, var(--tw-gradient-stops))',
        'gradient-hero': 'radial-gradient(circle at top, rgba(56,189,248,0.16), rgba(15,23,42,0.2), transparent 65%)',
      },
    },
  },
  plugins: [],
};
