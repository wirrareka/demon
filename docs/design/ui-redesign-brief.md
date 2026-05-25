# Kalista — UI / UX redesign brief

Prompt for a UI/UX design agent (or human designer). Paste this whole
file as the brief.

---

You are designing the admin UI for **Kalista**, an Apache-2.0
reverse proxy with built-in Let's Encrypt. The product is a Rust +
Pingora data plane (Cloudflare-class performance) paired with a
React + Vite + Tailwind + shadcn/ui admin surface. The current UI
ships and works, but it was built function-first by a backend
engineer — your job is to bring it to Linear / Vercel / Stripe /
Raycast craft level without burning the existing component stack.

## Audience

- **Primary**: small-team SRE, dev-platform engineers running 5–500
  internal services behind one proxy. Power users, keyboard-first,
  Cmd-K natives.
- **Secondary**: premium homelab operators who pick Caddy or Nginx
  Proxy Manager today and want both more polish and more depth.
- **Anti-persona**: enterprise procurement, beginner web admins.
  No marketing-driven dashboards, no onboarding walkthroughs that
  require six clicks to dismiss.

## Positioning

Kalista exists because the market is split — HAProxy / Envoy
deliver performance with brutalist UX; Nginx Proxy Manager /
Zoraxy deliver friendliness with Bootstrap-2018 visuals. We refuse
to pick. Pingora-class throughput **and** the kind of UI an
operator opens a fourth tab in their browser to keep visible.

Visual references (rank-order): **Linear** (information density +
keyboard-first), **Vercel** (auth + deploy / cert flows), **Stripe
Dashboard** (data tables + diff views), **Raycast** (Cmd-K),
**Arc** (sidebar density). Avoid Material, avoid every
"AI-generated dashboard" template, avoid Tailwind UI's
out-of-the-box marketing aesthetic.

## Design system constraints

Stay inside the existing toolchain — **do not** propose a new
component library or a custom design system from scratch:

- **shadcn/ui** primitives (Radix under the hood). All dialogs,
  popovers, tooltips, dropdowns, sheets, command palette are
  shadcn. Restyling is fair game; replacing is not.
- **Tailwind CSS** utility classes. No CSS-in-JS, no styled-
  components. Tailwind theme tokens go in `tailwind.config.js`
  and are referenced everywhere.
- **lucide-react** for icons. One icon style, no mixing.
- **Recharts** for charts (with the option of `uPlot` for the
  live-streaming sparklines only). No D3 from scratch.
- **TanStack Table** for tables. We won't hand-roll grids.
- Theming: **light / dark / system** (already shipped). Both
  modes must feel equally cared-for — dark mode is **not** a
  reskin of light, it's a first-class surface.
- Type scale: prefer Inter or the system stack — open to your
  recommendation if it's free for commercial use.
- Density: **compact by default**. 36 px row height in tables,
  not 56. Operators scan hundreds of rows.

## Screens to (re)design — in priority order

(All exist today. Audit the live UI by reading
`web/src/pages/*` + `web/src/components/*` and the screenshots in
`docs/design/screenshots/` if present.)

1. **App shell** — sidebar (collapsible), top bar, tenant
   switcher, command-palette entry, breadcrumbs, theme toggle,
   user menu. Critical: the sidebar must scale to 5+ tenants
   without becoming a scroll bar.
2. **Hosts list** — the most-trafficked screen. Dense table:
   hostname, upstream, cert status pill, last 5 status codes
   sparkline, cache chip, rate-limit chip, ACL chip. URL filter
   chips (`?q=`, `?status=`), bulk select, inline edit dialog
   trigger.
3. **Add / Edit Host wizard** — multi-step (Domain → Upstream
   → TLS → Headers → Cache → Rate limit → ACL → Review). The
   **Review step** is a Terraform-plan-style diff (added /
   changed / removed) — this is one of the product's UX
   differentiators. Mode: dialog (current) or full-page sheet —
   propose both, defend the choice.
4. **Host detail page** — header with status, key actions
   (Reissue cert, Force renew, Clone, Delete), tabs for
   Overview / Cert lifecycle (Gantt) / Live logs / Audit /
   Metrics. The cert lifecycle Gantt is a differentiator —
   timeline of issued → renewed → expires, with hover state
   showing ACME order URL + retries.
5. **Drafts** (Stage/Apply) — pending mutations with diff
   preview, batch apply with order awareness, discard-all.
   Status filter chips.
6. **Certificates** — global list + per-cert lifecycle Gantt
   detail, force renew button, custom upload flow.
7. **Upstreams** — list + per-upstream detail with active
   health-check graph.
8. **Live logs** — streaming table with filter chips, request
   inspector drawer (per-request waterfall: TLS, request
   header, upstream connect, upstream first byte, upstream
   complete, response complete).
9. **Audit log** — search + kind/date filters, CSV/JSONL
   export, deep-link to the related entity.
10. **Cmd-K palette** — fuzzy across hosts, upstreams, certs,
    actions ("Add host", "Force renew", "Tail logs for host
    X"). Keyboard nav indicators, recents, scoped search per
    section.
11. **Setup wizard** (first-run) — 3 steps: tenant → admin
    user → review. No skippable steps. Sealed after first
    success.
12. **Settings** — OIDC, SMTP, alerts, retention, backups
    (scheduled + manual), diagnostics. Tabbed.
13. **TCP routes**, **Tenants admin**, **Webhooks /
    Alerts** — secondary screens, ship at parity with
    Linear/Vercel form aesthetics.

## UX differentiators — preserve and amplify

These are what make the product fall through the gap in the
market. Do not water them down:

- **Stage / Apply diff** — Terraform-plan colour coding
  (green +, red -, yellow ~), side-by-side or unified at user
  pref.
- **Cert lifecycle Gantt** — horizontal timeline per cert,
  not a row in a CRUD table.
- **Cmd-K everywhere** — every page action reachable in two
  keystrokes.
- **Live request waterfall** — per-request inspector that
  shows the Pingora phases (Server-Timing header is already
  wired).
- **Density + scan speed** — operators visit Hosts 50× a day;
  every pixel of vertical chrome costs them.
- **`g h`, `g u`, `g c` keyboard nav** (already shipped) —
  preserve and document in a discoverable shortcuts modal.

## Deliverables I want

In this order:

1. **One-page design rationale** — what changed, why, what
   stayed. Plain markdown.
2. **Type + color tokens** — Tailwind theme additions
   (`tailwind.config.js` snippet ready to drop in). Light +
   dark. No more than 8 semantic colors total
   (background / surface / surface-elevated / border / muted /
   accent / success / danger), each with explicit dark mode
   pairings.
3. **Component-level redlines** for the 6 most-trafficked
   surfaces (App shell, Hosts list, Add Host wizard incl.
   Review step, Host detail with cert Gantt, Drafts, Cmd-K).
   Format: high-fidelity mockups (Figma or `.pen` via Pencil
   MCP, your call). Each surface ships at three viewports —
   1280 × 800, 1440 × 900, 1920 × 1080.
4. **Empty states** for every list view — what does Hosts
   look like with zero hosts after the setup wizard? With one?
   With 500?
5. **Loading + error states** for the four async-heavy
   surfaces (Hosts, Logs, Drafts, Certs).
6. **Motion spec** — what eases, when. We use Radix's defaults
   today; you may override but justify each.

## Constraints — please honour

- **No new dependencies** without naming them + the size cost.
- **Accessibility**: WCAG 2.2 AA minimum. Contrast ratios,
  focus rings (not just outline-none), reduced-motion
  fallbacks, keyboard reach for every interactive element.
- **i18n**: the app ships in en/sk/cs (more later). Avoid
  fixed-width layouts that break on long German / Hungarian
  strings.
- **No proprietary fonts** behind paywalls. Inter / IBM Plex /
  system stack is fine.
- **No emoji as icons.** Lucide only.

## What to read before designing

- `CLAUDE.md` — full repo orientation
- `docs/research/competitive-landscape.md` — why this product
  exists, who we're up against
- `docs/research/ux-landscape.md` — JTBDs, IA, ASCII
  wireframes the eng team worked from
- `docs/architecture/0002-multi-tenant-data-model.md` — how
  tenants compose in the URL hierarchy
- `CHANGELOG.md` — what's actually shipped through v0.4.76
- `web/src/pages/*` — current implementation to audit
- `docs-site/src/content/docs/operate/*` — operator language
  + mental model

## Deliverable format

- Put rationale + tokens in `docs/design/redesign-rationale.md`
  + `docs/design/tokens.md`.
- Put redlines in `docs/design/screens/` (one folder per
  surface; PNG/SVG/PDF + a 1-paragraph spec per screen).
- Open a PR titled `design: v0.5 redesign brief response` so
  engineering can comment inline.

## What I will reject

- Marketing-style hero sections on operator screens.
- Mocks with placeholder Lorem Ipsum — use realistic
  operator-language strings ("api.prod.example.com",
  "stripe-webhook-receiver", "443 → 8080", not "Item One").
- Mocks with 6 rows in tables where production will see 500.
  Show the dense case.
- Glassmorphism, neumorphism, gradient borders, animated
  backgrounds. Operators run this on a second monitor.
- Anything that requires explaining what an icon means in a
  tooltip on first hover. Either the icon reads or it has a
  label.

Push back on anything in this brief that's wrong. The brief is
a starting position, not a spec.
