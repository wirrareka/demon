"use strict";Object.defineProperty(exports, "__esModule", {value: true});

/**
 * Wires Tailwind / shadcn color keys onto the kalista v0.5 OKLCH CSS variables
 * (see src/styles/tokens.css), so utilities and components inherit the design system.
 */
exports. default = {
  darkMode: ["class", '[data-theme="dark"]'],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: "var(--bg)",
        surface: "var(--surface)",
        "surface-2": "var(--surface-2)",
        "surface-3": "var(--surface-3)",
        border: "var(--border)",
        "border-2": "var(--border-2)",
        ring: "var(--ring)",
        fg: "var(--fg)",
        "fg-2": "var(--fg-2)",
        muted: "var(--muted)",
        "muted-2": "var(--muted-2)",
        accent: { DEFAULT: "var(--accent)", fg: "var(--accent-fg)", soft: "var(--accent-soft)" },
        success: { DEFAULT: "var(--success)", bg: "var(--success-bg)", border: "var(--success-border)", fg: "var(--success-fg)" },
        warning: { DEFAULT: "var(--warning)", bg: "var(--warning-bg)", border: "var(--warning-border)", fg: "var(--warning-fg)" },
        danger: { DEFAULT: "var(--danger)", bg: "var(--danger-bg)", border: "var(--danger-border)", fg: "var(--danger-fg)" },
        info: { DEFAULT: "var(--info)", bg: "var(--info-bg)", border: "var(--info-border)", fg: "var(--info-fg)" },
      },
      borderRadius: {
        sm: "var(--radius-sm)",
        md: "var(--radius-md)",
        lg: "var(--radius-lg)",
        xl: "var(--radius-xl)",
      },
      fontFamily: {
        sans: "var(--font-sans)",
        mono: "var(--font-mono)",
      },
      boxShadow: {
        1: "var(--shadow-1)",
        2: "var(--shadow-2)",
        pop: "var(--shadow-pop)",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
} ;
 /* v7-bf51d9cf94b9a488 */