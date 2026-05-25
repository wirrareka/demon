import type { Config } from "tailwindcss";
import animate from "tailwindcss-animate";

// Kalista visual standard — see docs/research/ux-landscape.md and the v0.5
// design foundation at docs/design/v0.5/project/tokens.md.
//
// As of v0.4.78 the v0.5 design tokens live in src/styles/tokens.css as raw
// CSS custom properties (no HSL channels — they're OKLCH literals). The
// shadcn-style keys (`background`, `foreground`, `border`, `accent`, etc.)
// are *rewired* below to read those same v0.5 vars, so every existing
// shadcn-built component picks up the new ladder without per-component edits.
// Surface restyles land in subsequent phases.
//
// `darkMode` is keyed off both `class="dark"` (so legacy refs keep working)
// and `[data-theme="dark"]` (the v0.5 selector the tokens.css block uses).
const config = {
  darkMode: ["variant", ["&:where(.dark, .dark *)", "&:where([data-theme='dark'], [data-theme='dark'] *)"]],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    container: {
      center: true,
      padding: "1rem",
      screens: { "2xl": "1400px" },
    },
    extend: {
      colors: {
        // ── shadcn keys, rewired to v0.5 vars. Existing components keep
        // calling `bg-background` / `text-foreground` / `border-border`
        // and quietly inherit the new ladder. `--input` and `--popover`
        // didn't exist in v0.5 so they map to the nearest surface step. ──
        border: "var(--border)",
        input: "var(--border)",
        ring: "var(--ring)",
        background: "var(--bg)",
        foreground: "var(--fg)",
        primary: {
          DEFAULT: "var(--fg)",
          foreground: "var(--bg)",
        },
        secondary: {
          DEFAULT: "var(--surface-2)",
          foreground: "var(--fg)",
        },
        destructive: {
          DEFAULT: "var(--danger)",
          foreground: "var(--danger-bg)",
        },
        muted: {
          DEFAULT: "var(--surface-2)",
          foreground: "var(--muted)",
        },
        accent: {
          DEFAULT: "var(--accent)",
          foreground: "var(--accent-fg)",
        },
        popover: {
          DEFAULT: "var(--surface)",
          foreground: "var(--fg)",
        },
        card: {
          DEFAULT: "var(--surface)",
          foreground: "var(--fg)",
        },

        // ── v0.5-native aliases. These are what the new surface restyles
        // will reach for; older surfaces keep using the shadcn names above. ──
        bg: "var(--bg)",
        surface: "var(--surface)",
        "surface-2": "var(--surface-2)",
        "surface-3": "var(--surface-3)",
        // v0.4.82 polish: dedicated row-hover step (was faked with surface-3/70).
        "row-hover": "var(--row-hover)",
        "border-2": "var(--border-2)",
        "breadcrumb-sep": "var(--breadcrumb-sep)",
        fg: "var(--fg)",
        "fg-2": "var(--fg-2)",
        "muted-2": "var(--muted-2)",
        "accent-fg": "var(--accent-fg)",
        "accent-soft": "var(--accent-soft)",

        // ── Status hues (and their bg / border partners). Single-token
        // form is the foreground colour, matching the v0.5 tokens.md naming. ──
        success: "var(--success)",
        "success-bg": "var(--success-bg)",
        "success-border": "var(--success-border)",
        // v0.4.82 polish: foreground tone for text/icons on `*-bg` chips
        // (the bare `*` is the dark-on-dark-surface value).
        "success-fg": "var(--success-fg)",
        warning: "var(--warning)",
        "warning-bg": "var(--warning-bg)",
        "warning-border": "var(--warning-border)",
        "warning-fg": "var(--warning-fg)",
        danger: "var(--danger)",
        "danger-bg": "var(--danger-bg)",
        "danger-border": "var(--danger-border)",
        "danger-fg": "var(--danger-fg)",
        info: "var(--info)",
        "info-bg": "var(--info-bg)",
        "info-border": "var(--info-border)",
        "info-fg": "var(--info-fg)",

        // ── Diff palette (consumed by the wizard Review step + Drafts page). ──
        "diff-add-bg": "var(--diff-add-bg)",
        "diff-add-fg": "var(--diff-add-fg)",
        "diff-add-gutter": "var(--diff-add-gutter)",
        "diff-rem-bg": "var(--diff-rem-bg)",
        "diff-rem-fg": "var(--diff-rem-fg)",
        "diff-rem-gutter": "var(--diff-rem-gutter)",
        "diff-mod-bg": "var(--diff-mod-bg)",
        "diff-mod-fg": "var(--diff-mod-fg)",
        "diff-mod-gutter": "var(--diff-mod-gutter)",
      },
      fontFamily: {
        sans: ["Inter", "ui-sans-serif", "system-ui", "sans-serif"],
        mono: ['"JetBrains Mono"', "ui-monospace", "SFMono-Regular", "monospace"],
      },
      // v0.5 type scale. The legacy `text-sm` (Tailwind default 14px) shifts
      // *down* to 12.5px to match the operator-dense layout. We re-checked
      // shadcn primitives (button, input, dialog) and 12.5px reads correctly.
      fontSize: {
        "2xs": ["10.5px", "1.2"],
        xs:    ["11.5px", "1.4"],
        sm:    ["12.5px", "1.45"],
        base:  ["13px",   "1.45"],
        md:    ["14px",   "1.4"],
        lg:    ["15px",   "1.3"],
        xl:    ["18px",   "1.2"],
      },
      borderRadius: {
        // shadcn-compatible `lg / md / sm` keys + v0.5 extensions.
        lg: "var(--radius-lg)",
        md: "var(--radius-md-plus)",
        sm: "var(--radius-sm)",
        DEFAULT: "var(--radius-md)",
        xl: "var(--radius-xl)",
      },
      boxShadow: {
        1: "var(--shadow-1)",
        2: "var(--shadow-2)",
        pop: "var(--shadow-pop)",
      },
      transitionTimingFunction: {
        ds: "cubic-bezier(.2,.7,.3,1)",
      },
      keyframes: {
        "accordion-down": {
          from: { height: "0" },
          to: { height: "var(--radix-accordion-content-height)" },
        },
        "accordion-up": {
          from: { height: "var(--radix-accordion-content-height)" },
          to: { height: "0" },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.18s cubic-bezier(.2,.7,.3,1)",
        "accordion-up": "accordion-up 0.18s cubic-bezier(.2,.7,.3,1)",
      },
    },
  },
  plugins: [animate],
} satisfies Config;

export default config;
