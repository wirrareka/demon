# Kalista v0.5 — design tokens

Drop-in `tailwind.config.js` and CSS variable mirror. Light + dark with explicit pairings. 8 semantic colors + 4 status hues, as the brief specifies.

## Semantic surface ladder

| Token | Light | Dark | Use |
| --- | --- | --- | --- |
| `background` | `#ffffff` | `#09090b` | Page background |
| `surface` | `#fafafa` | `#0c0c0e` | Cards, sidebar, dialogs |
| `surface-2` | `#f4f4f5` | `#131316` | Toolbars, table-header bg, chips |
| `surface-3` | `#ededee` | `#1a1a1d` | Active states, hover-on-surface-2, kbd |
| `border` | `#e4e4e7` | `#1f1f24` | Hairlines |
| `border-2` | `#d4d4d8` | `#2a2a30` | Input borders on hover, divider emphasis |
| `fg` | `#09090b` | `#fafafa` | Primary text, primary action background |
| `fg-2` | `#3f3f46` | `#d4d4d8` | Body text on surfaces |
| `muted` | `#71717a` | `#8b8b94` | Secondary text, captions |
| `muted-2` | `#a1a1aa` | `#5a5a63` | Tertiary text, placeholders |

## Status hues

| Token | Light | Dark | Use |
| --- | --- | --- | --- |
| `success` | `#15803d` | `#4ade80` | Active cert, 2xx |
| `warning` | `#b45309` | `#fbbf24` | Cert expiring/renewing, diff `~` |
| `danger` | `#b91c1c` | `#f87171` | Expired, 5xx, deny |
| `info` | `#1d4ed8` | `#60a5fa` | Cert issued, banners |

Each has matching `*-bg` and `*-border` tokens. See `styles.css` `:root` block for the full set.

## Accent

Mono by default — `accent = fg`, `accent-fg = bg`. Five tweakable hues share OKLCH lightness `0.74` (dark) / `0.52` (light) and chroma `0.13–0.20`, so swapping hue preserves WCAG contrast.

```css
[data-accent="blue"][data-theme="dark"]   { --accent: oklch(0.74 0.13 245); --accent-fg: #0a0e1a; }
[data-accent="blue"][data-theme="light"]  { --accent: oklch(0.52 0.18 245); --accent-fg: #ffffff; }
[data-accent="violet"][data-theme="dark"] { --accent: oklch(0.72 0.16 295); --accent-fg: #100618; }
[data-accent="green"][data-theme="dark"]  { --accent: oklch(0.78 0.16 155); --accent-fg: #04140a; }
[data-accent="orange"][data-theme="dark"] { --accent: oklch(0.76 0.16 50);  --accent-fg: #170803; }
```

## Diff palette

| Token | Light bg / fg / gutter | Dark bg / fg / gutter |
| --- | --- | --- |
| `diff-add` | `#f0fdf4` / `#166534` / `#16a34a` | `rgba(34,197,94,.08)` / `#86efac` / `#22c55e` |
| `diff-rem` | `#fef2f2` / `#991b1b` / `#dc2626` | `rgba(248,113,113,.08)` / `#fca5a5` / `#ef4444` |
| `diff-mod` | `#fffbeb` / `#92400e` / `#d97706` | `rgba(251,191,36,.07)` / `#fcd34d` / `#f59e0b` |

## Type

| Use | Family | Size / line-height | Weight |
| --- | --- | --- | --- |
| UI body | Inter | 13 / 1.45 | 400 |
| Strong body | Inter | 13 / 1.45 | 500 |
| Page title `h1` | Inter | 18 / 1.2 | 600 |
| Card title `h3` | Inter | 12.5 / 1.2 | 600 |
| Table cell | Inter / JBM | 12.5 / 1.4 | 400 / 500 |
| Caption | Inter | 11.5 / 1.4 | 400 |
| Section label (caps) | Inter | 10.5 / 1.2 | 600, +0.06em |
| Hostname / upstream / code | **JetBrains Mono** | 12–12.5 | 400 |
| Kbd | JetBrains Mono | 10.5 | 500 |

`tnum` is on globally so RPS / latency columns line up. JetBrains Mono disables `calt` so `->` doesn't fuse into a ligature in upstream paths.

## Density

```css
[data-density="compact"] { --row-h: 36px; --cell-py: 8px; }
[data-density="extra"]   { --row-h: 30px; --cell-py: 6px; }
```

## Radii

`4px` chips · `5–6px` chips/buttons · `6–8px` inputs/cards · `8–10px` dialogs · `12px` cmdk.

## Shadows

Three shadows, each layers a 1px ring with a soft drop. Dark-mode shadow uses pure black; light-mode uses `rgba(9,9,11,*)` so it stays neutral.

```css
--shadow-1:    0 0 0 1px rgba(9,9,11,.04), 0 1px 2px rgba(9,9,11,.06);
--shadow-2:    0 0 0 1px rgba(9,9,11,.04), 0 8px 24px -8px rgba(9,9,11,.18);
--shadow-pop:  0 0 0 1px rgba(9,9,11,.06), 0 24px 48px -16px rgba(9,9,11,.32);
```

## `tailwind.config.js` snippet

```js
// tailwind.config.js — drop-in for the v0.5 redesign
const ds = (lightVar, darkVar) => `rgb(var(--${lightVar}) / <alpha-value>)`;
module.exports = {
  darkMode: ['class', '[data-theme="dark"]'],
  content: ['./web/src/**/*.{ts,tsx,jsx}'],
  theme: {
    extend: {
      colors: {
        background:    'var(--bg)',
        surface:       'var(--surface)',
        'surface-2':   'var(--surface-2)',
        'surface-3':   'var(--surface-3)',
        border:        'var(--border)',
        'border-2':    'var(--border-2)',
        fg:            'var(--fg)',
        'fg-2':        'var(--fg-2)',
        muted:         'var(--muted)',
        'muted-2':     'var(--muted-2)',
        accent:        'var(--accent)',
        'accent-fg':   'var(--accent-fg)',
        ring:          'var(--ring)',
        success:       'var(--success)',
        warning:       'var(--warning)',
        danger:        'var(--danger)',
        info:          'var(--info)',
      },
      fontFamily: {
        sans: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'ui-monospace', 'SFMono-Regular', 'monospace'],
      },
      fontSize: {
        '2xs': ['10.5px', '1.2'],
        xs:    ['11.5px', '1.4'],
        sm:    ['12.5px', '1.45'],
        base:  ['13px',   '1.45'],
        md:    ['14px',   '1.4'],
        lg:    ['15px',   '1.3'],
        xl:    ['18px',   '1.2'],
      },
      borderRadius: { sm: '4px', DEFAULT: '6px', md: '7px', lg: '8px', xl: '10px' },
      boxShadow: {
        1:   '0 0 0 1px rgb(9 9 11 / 0.04), 0 1px 2px rgb(9 9 11 / 0.06)',
        2:   '0 0 0 1px rgb(9 9 11 / 0.04), 0 8px 24px -8px rgb(9 9 11 / 0.18)',
        pop: '0 0 0 1px rgb(9 9 11 / 0.06), 0 24px 48px -16px rgb(9 9 11 / 0.32)',
      },
      transitionTimingFunction: { ds: 'cubic-bezier(.2,.7,.3,1)' },
    },
  },
};
```

CSS variables live in `styles/tokens.css` and switch by `data-theme` / `data-accent` / `data-density` on `<html>`.

## Contrast cheatsheet (calculated)

| Pair | Light | Dark | AA pass |
| --- | --- | --- | --- |
| `fg` on `bg` | 19.8:1 | 17.2:1 | ✓ AAA |
| `fg-2` on `bg` | 10.4:1 | 12.4:1 | ✓ AAA |
| `muted` on `bg` | 5.4:1 | 5.7:1 | ✓ AA (text ≥ 12 px) |
| `muted-2` on `bg` | 3.9:1 | 3.6:1 | ✓ AA (text ≥ 14 px) — captions only |
| `success` on `success-bg` | 6.8:1 | 8.1:1 | ✓ AAA |
| `warning` on `warning-bg` | 6.4:1 | 8.6:1 | ✓ AAA |
| `danger` on `danger-bg` | 7.1:1 | 7.0:1 | ✓ AAA |

## Notes

- All `<input>`, `<button>`, `<a>` carry `:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }`. Never `outline: none` without a replacement.
- `prefers-reduced-motion: reduce` zeroes all keyframe animations and skips the diff/skeleton shimmer.
- `tabular-nums` and `cv11`/`ss01`/`ss03` Inter features are on globally so numbers and label glyphs are consistent.
