# proximiio.demon — UI/UX design package (vendored from kalista)

This is the **binding UI/UX design package for proximiio.demon's web UI**, brought in
from **kalista** (`quanto/kalista/docs/design/` + `quanto/kalista/web/`). Demon's web
UI (Phase 7) and any styled thin client follow this system. Demon **keeps the design
system** (tokens, layout, component patterns) and only **swaps branding/content** for
its own surfaces (fleet/host/jobs/alerts/audit/runbooks/tenants).

> Source of truth note: kalista owns the evolution of this design system. This is a
> point-in-time vendored copy (kalista v0.5). When kalista's design moves, re-vendor
> and re-check against `03-ui-design-system.md`.

## What's here

| Path | What it is |
|---|---|
| `ui-redesign-brief.md` | The redesign brief — goals, principles, direction |
| `v0.5/README.md`, `v0.5/CONTRAST-VERIFIED.md` | v0.5 package overview + WCAG contrast proof |
| `v0.5/project/tokens.md` | **Design tokens** (the canonical spec) |
| `v0.5/project/styles.css` | Full stylesheet (OKLCH vars, surface ladder, status hues) |
| `v0.5/project/redesign-rationale.md` | Why the system is shaped this way |
| `v0.5/project/*.jsx`, `data.js`, `index.html`, `logo.html` | Reference prototype (shell, surfaces, cmdk, wizard, tweaks panel) — read for layout/interaction patterns, not copied verbatim |
| `traffic-inspect-traps-ratelimit.md` | Kalista-feature-specific design — reference only (not a demon surface) |
| `web-assets/tokens.css` | **Live token CSS** — copy this verbatim into demon `web/src/styles/` |
| `web-assets/index.css`, `tailwind.config.ts`, `components.json`, `postcss.config.js` | Live Tailwind + shadcn config to mirror in demon `web/` |

## How to use (Phase 7)

1. Scaffold demon `web/` (React + Vite + Tailwind + shadcn) and copy `web-assets/`
   tokens + config verbatim (OKLCH vars, `data-theme`/`data-accent`/`data-density`,
   default dark/mono/compact).
2. Mirror `components.json` (`new-york`, `baseColor neutral`, `cssVariables true`,
   lucide) and the `tailwind.config.ts` var-rewire.
3. Lift the shadcn primitives from kalista `web/src/components/ui/` (alert-dialog,
   badge, button, card, command, dialog, input, label, select, spinner, table, Toaster).
4. Build to `docs/planning/03-ui-design-system.md` — it maps these tokens/patterns onto
   demon's screens and lists the **demon-specific deltas** (typed-confirm dual-control
   modal, dry-run diff view, EU/UAE residency switcher, WS live-state indicators,
   canonical `lib/health.ts`, fleet stat tiles).
