# proximiio.demon — UI Design System Spec

**Status:** binding. **Source of truth:** KALISTA v0.5 design system
(`/Volumes/ExtremeSSD/quanto/kalista`). demon's web UI (React + Vite + Tailwind +
shadcn/ui) MUST match kalista's visual standard. This doc is what a build agent
implements against — every token name/value below is copied verbatim from kalista
so the two products are visually one family.

Toolchain (locked, do not substitute): React 18, Vite, Tailwind, **shadcn/ui
new-york**, **lucide-react** icons, **TanStack Table**, **Recharts** (+ uPlot for
live sparklines only). No CSS-in-JS, no Material, no replacing Radix primitives —
restyle, never replace.

Target persona: keyboard-first SRE / platform operators. Information-dense,
Cmd-K-native, dark-mode-first. References: Linear, Vercel, Stripe, Raycast.

---

## 1. Design tokens

Tokens live in **`src/styles/tokens.css`** as raw **OKLCH literals** on CSS custom
properties (NOT HSL channels). They switch by attributes on `<html>`:
`data-theme="dark|light"`, `data-accent="mono|cobalt|solder|pingora|iris"`,
`data-density="compact|extra"`. **Default = dark / mono / compact.** Never reintroduce
an `hsl(var(--x))` block in `index.css` — it shadows the OKLCH ladder.

### 1.1 Surface ladder (10 semantic levels)

| Token (`--`)   | Light (hex ref) | Dark (hex ref) | Use |
| --- | --- | --- | --- |
| `bg`        | `#ffffff` | `#09090b` | Page background |
| `surface`   | `#fafafa` | `#0c0c0e` | Cards, sidebar, dialogs |
| `surface-2` | `#f4f4f5` | `#131316` | Toolbars, table-header bg, chips |
| `surface-3` | `#ededee` | `#1a1a1d` | Active / hover-on-surface-2, kbd |
| `row-hover` | between surf-2/3 | between surf-2/3 | dedicated table/nav row hover |
| `border`    | `#e4e4e7` | `#1f1f24` | Hairlines |
| `border-2`  | `#d4d4d8` | `#2a2a30` | Input hover borders, divider emphasis |
| `fg`        | `#09090b` | `#fafafa` | Primary text + primary-action bg |
| `fg-2`      | `#3f3f46` | `#d4d4d8` | Body text on surfaces |
| `muted`     | `#71717a` | `#8b8b94` | Secondary text, captions (AA ≥12px) |
| `muted-2`   | `#a1a1aa` | `#5a5a63` | Tertiary / placeholders (captions only) |

Exact OKLCH values are in kalista `src/styles/tokens.css` — copy that file 1:1.
Light `--fg` = `oklch(0.131 0.012 250)` (19.81:1 AA), dark `--fg` =
`oklch(0.975 0.003 250)`. `--breadcrumb-sep` named separately.

### 1.2 Status hues — CRITICAL for demon (health/up/down/degraded/unknown)

Each hue has 4 tokens: bare `--x` (fg on dark surface), `--x-bg`, `--x-border`,
`--x-fg` (fg-on-`*-bg`). Tailwind exposes all four as `success / success-bg /
success-border / success-fg` etc.

| Token | Light | Dark | Meaning in demon |
| --- | --- | --- | --- |
| `success` | `#15803d` | `#4ade80` | **up / healthy** |
| `warning` | `#b45309` | `#fbbf24` | **degraded / renewing / pending** |
| `danger`  | `#b91c1c` | `#f87171` | **down / failed / deny** |
| `info`    | `#1d4ed8` | `#60a5fa` | **issued / banners / informational** |

**Demon health-state → token mapping (NEW, define in `lib/health.ts`):**
`up/healthy → success`, `degraded → warning`, `down/critical → danger`,
`unknown → muted` (neutral grey, NOT a status hue), `pending/syncing → info`.
Render via `.pill` + `.dot` (6px) so colour + a literal dot both encode state
(colour-blind safe).

### 1.3 Diff palette (for dry-run / config diff views)

`diff-add` (green), `diff-rem` (red), `diff-mod` (amber) each with `-bg / -fg /
-gutter`. Copy from kalista tokens.css; consumed via Tailwind `bg-diff-add-bg`,
`text-diff-add-fg`, gutter `text-diff-add-gutter`.

### 1.4 Typography

Families: **Inter** (`--font-sans`), **JetBrains Mono** (`--font-mono`, for
hostnames / IPs / service IDs / code / job IDs). Globally on `<html>`:
`font-feature-settings: 'tnum','cv11','ss01','ss03'` — tabular nums so RPS /
latency / count columns align everywhere without per-cell opt-in.

Scale (Tailwind `fontSize`): `2xs 10.5/1.2` · `xs 11.5/1.4` · `sm 12.5/1.45` ·
`base 13/1.45` · `md 14/1.4` · `lg 15/1.3` · `xl 18/1.2`. Note `sm` is 12.5px
(operator-dense), not Tailwind default 14. Page title `h1` = `xl 600`; section
caps label = `2xs 600 +0.06em`.

### 1.5 Radius / shadow / motion / density

Radii vars: `--radius-sm 4px`, `--radius-md 6px` (DEFAULT), `--radius-md-plus 7px`
(shadcn `md`), `--radius-lg 8px`, `--radius-xl 10px`; cmdk uses 12px.
Shadows: `--shadow-1` (1px ring + soft drop), `--shadow-2`, `--shadow-pop`
(dialogs/popovers); dark uses pure-black, light uses neutral `rgba(20,18,26,*)`.
Tailwind keys: `shadow-1 / shadow-2 / shadow-pop`.
Motion easing: `cubic-bezier(.2,.7,.3,1)` (Tailwind `ease-ds`). Honour
`prefers-reduced-motion` (zero animations).
Density: `[data-density=compact]` `--row-h:36px --row-h-header:32px`;
`[data-density=extra]` `--row-h:30px`. Accent themes (mono default + 4 hues)
share an OKLCH lightness band so swapping preserves contrast.

---

## 2. Tailwind + shadcn setup demon must mirror

**`components.json`** (copy exactly):
```json
{ "style": "new-york", "rsc": false, "tsx": true,
  "tailwind": { "config": "tailwind.config.ts", "css": "src/index.css",
    "baseColor": "neutral", "cssVariables": true, "prefix": "" },
  "aliases": { "components":"@/components", "utils":"@/lib/utils",
    "ui":"@/components/ui", "lib":"@/lib", "hooks":"@/hooks" },
  "iconLibrary": "lucide" }
```

**`tailwind.config.ts`** — mirror kalista exactly. Key points:
- `darkMode: ["variant", ["&:where(.dark, .dark *)", "&:where([data-theme='dark'], [data-theme='dark'] *)"]]`
- **Rewire shadcn keys onto v0.5 vars** so stock shadcn components inherit the
  ladder with zero per-component edits: `border→--border`, `input→--border`,
  `ring→--ring`, `background→--bg`, `foreground→--fg`, `primary→{--fg,--bg}`,
  `secondary→{--surface-2,--fg}`, `destructive→{--danger,--danger-bg}`,
  `muted→{--surface-2,--muted}`, `accent→{--accent,--accent-fg}`,
  `popover/card→{--surface,--fg}`.
- Add v0.5-native aliases: `bg, surface, surface-2, surface-3, row-hover,
  border-2, breadcrumb-sep, fg, fg-2, muted-2, accent-fg, accent-soft`, all status
  `*/-bg/-border/-fg`, all `diff-*`.
- `fontFamily`, `fontSize`, `borderRadius` (`lg/md/sm/DEFAULT/xl`→radius vars),
  `boxShadow 1/2/pop`, `transitionTimingFunction.ds`, accordion keyframes.
- Plugins: `tailwindcss-animate`.

**Required base shadcn components** (port kalista's restyled versions, do not
re-init from upstream): `button, badge, card, dialog, alert-dialog, input, label,
select, table, command, spinner, Toaster`. `badge` uses a `cva("pill", …)` with
variants `default/secondary/destructive(danger)/outline(ghost)/success/warning`.

**Global CSS classes** to port from kalista `styles.css`: `.pill` (+ `.success
.warning .danger .info .ghost .mono` and child `.dot`), `.btn` (+ `.primary
.ghost .danger .sm .icon`), `.kbd`, `.side-item/.side-section`, `.cmdk-*`, and the
`.kalista-cb` styled checkbox (rename `demon-cb`). Focus-visible everywhere:
`outline:2px solid var(--ring); outline-offset:2px`.

---

## 3. Reusable layout / component patterns (port from kalista)

- **App shell** (`components/layout/`): `AppShell` (grid: sidebar + Topbar +
  Outlet), `Sidebar` (sectioned nav with caps `.lbl` headers, lucide icons,
  `g _` keyboard-hint kbd chips, collapse persisted to localStorage +
  `data-side` on `<html>`), `Topbar`, `PageEnvelope` (page title + actions slot),
  `RouteFallback`, `KeyboardHints`. **TenantSwitcher** lives at sidebar top —
  directly reusable for demon's tenant scoping.
- **Command palette** (`CommandPalette.tsx` + `KeyboardHelp.tsx`): Raycast-style,
  160ms `cmdk-pop-in`. demon registers fleet/host/job/runbook navigation + actions.
- **Data tables** (TanStack Table): sticky header at `--row-h-header` (32px), body
  rows `--row-h` (36px), `row-hover` bg, mono entity column, `tnum` numerics,
  bulk-select checkbox column, `↑/↓/↩` keyboard nav, "pin/filter to this entity"
  hover action. Pattern in `HostsPage`/`AuditPage` → reuse for demon host &
  service inventory.
- **Detail pane** (`HostDetailPage`): definition lists (`dt/dd`), status badges,
  inline action buttons gated by `useConfirm`. → demon host/service detail.
- **Dialogs**: shadcn `dialog` (forms, `shadow-pop`) + `alert-dialog` via the
  `useConfirm()` hook returning `[confirm, confirmUI]`; supports
  `{title, description, confirmLabel, destructive}`. → demon mutation confirms.
- **Audit view** (`AuditPage`): sticky-header table, mono target column,
  action-verb `.pill` coloured by create/update/delete, "show only this entity"
  pin. → demon audit log directly.
- **Status badges**: `.pill` + `.dot`; `<Badge variant=…>` for inline cell states.
- **Toasts**: `components/ui/Toaster.tsx` for mutation success/failure.
- **Diff**: `components/diff/HostConfigDiff.tsx` using the `diff-*` palette →
  base for demon dry-run diff.
- **Charts**: Recharts themed to tokens; uPlot for live sparklines.

### demon screen → kalista source map

| demon screen | reuse from kalista |
| --- | --- |
| Fleet overview | `OverviewPage` (cards + Recharts + status tiles) |
| Host / service health | `HostsPage` table + `HostDetailPage` pane; status pills |
| Jobs | TanStack table pattern + status pills + detail pane |
| Alerts | `AlertsPage` (severity pills, ack flow) |
| Audit | `AuditPage` (1:1) |
| Runbooks | new content in `PageEnvelope` + table shell |
| Tenants | `TenantsAdminPage` + sidebar `TenantSwitcher` |

---

## 4. Delta — NEW patterns demon needs (not in kalista)

1. **Typed-confirm dual-control modal** — for high-blast-radius mutations
   (restart fleet, drain host, rotate creds). Extends `useConfirm`: require the
   operator to type the exact resource name/token (`requireText: string`),
   disable confirm until match, `destructive` styling. Optionally a second
   approver field (dual-control). Build as `ConfirmDangerous`/`useTypedConfirm`.
2. **Dry-run diff view** — preview a planned change before apply. Reuse the
   `diff-*` palette + `HostConfigDiff` layout, but as a modal/sheet step with
   add/remove/modify counts and an explicit "Apply" gated by typed-confirm.
3. **Residency switcher (EU / UAE)** — a top-bar segmented control writing a
   `data-residency` attr + scoping all API calls; visually a sibling of the
   accent switcher. Persisted like sidebar collapse. Show a residency chip in the
   topbar at all times (compliance affordance).
4. **WebSocket live-state indicators** — a connection-status dot in the topbar
   (`success`=live, `warning`=reconnecting, `danger`=offline using the status
   tokens), plus per-row "live" pulse on tables fed by WS. Add a subtle
   `live-pulse` keyframe (respect reduced-motion). Optimistic UI + conflict toast.
5. **Health-state token map** (`lib/health.ts`) — canonical
   up/degraded/down/unknown/pending → token mapping (see §1.2); demon leans on
   status hues far more heavily than kalista, so centralise it.
6. **Fleet density / aggregate tiles** — overview stat tiles summarising N hosts
   by health (counts of up/degraded/down) using status `*-bg`/`*-fg` chips.
