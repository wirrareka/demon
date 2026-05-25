# Kalista v0.5 — redesign rationale

A one-page response to the brief. What changed, why, what stayed.

## What stayed

- **Stack.** shadcn/ui primitives, Tailwind, lucide-react, Recharts, TanStack Table, light/dark/system, `g h` `g u` `g c` shortcuts. No new dependencies.
- **Information architecture.** Same sidebar groupings, same surface set, same wizard step count. The redesign is a craft pass over the same product, not a re-org.
- **Compact-by-default.** Brief asks for 36 px rows. We hit 36 px and add an `extra` density at 30 px for power users.
- **shadcn restyling, not replacement.** Buttons, inputs, pills, popovers, dialogs, command palette — all match the brief's primitive set.

## What changed

### Type system
- **Inter** for UI; **JetBrains Mono** for every hostname, upstream, header value, audit message, and cert serial. Hostnames are the primary nouns of this product — they deserve their own face. Inter's `tnum` is on globally so columns line up.
- One type scale: 11 / 11.5 / 12 / 12.5 / 13 / 14 / 15 / 16 / 18. No 20 / 24 / 32 — operator screens have no need.

### Color
- **Mono accent by default.** Primary action is `--fg` over `--fg-2` background. Saves the chromatic accents for the only places they earn rent: cert status (green/amber/red), diff gutters (green/red/amber), 5xx error class.
- **Accent hue tweakable** (mono/blue/violet/green/orange) for teams that want a chromatic primary. Same OKLCH lightness and chroma per hue so swapping doesn't break contrast targets.
- **Dark and light are designed independently.** Dark surface ladder steps `#09 → #0c → #13 → #1a` instead of a uniform tint. Borders are 1.5 lightness steps darker than the surface they sit on — not a translucent overlay.
- **8 semantic colors max.** `background / surface / surface-2 / surface-3 / border / fg / fg-2 / muted` plus 4 status hues. Each has explicit dark pairings. See `tokens.md`.

### Hosts list
- **Real density.** 36 px rows, sticky header, 528-row count surfaced in the page header so you remember the production scale. Sparkline column shows last-5 status buckets, color-encoded by worst-class.
- **Filter chips, not a filter panel.** Five chips cover ~95 % of operator queries (all, cert ≤14d, 5xx, cache, rate-limited). The "More filters" escape hatch is still there for the long tail.
- **Bulk action bar floats in.** Selection bar appears bottom-center on selection — Linear-style — so the table never moves and the row count footer stays put.
- **Lock glyph on hostname.** TLS state visible in the leftmost glance, even before the cert pill.

### Add / Edit Host wizard
- **Two formats, defended.** Create is a **full-page sheet**, edit is a **dialog**. Rationale: create is a 7-step linear task, dialog clipping is painful; edit usually means changing one field, full-page is overkill. Both modes share one component.
- **Review step is the differentiator.** Terraform-plan diff with `+ / − / ~` gutter marks, unified by default, split on toggle. Hunk headers (`@@ host "api.prod.acme.com"`) anchor what changed where. `# was "old"` inline comments on `~` lines show old values without forcing split.
- **Each step is a single concept.** Domain, Upstream, TLS, Headers, Cache, Rate limit, ACL, Review. No "advanced" tab hiding what should be visible.
- **`⌘↵` continues or stages.** Power-user path is keyboard-only; no need to chase the Continue button.

### Host detail + Cert lifecycle Gantt
- **The Gantt is one lane per cert version, not per host.** Each version's lifecycle (issued → active → renewed/replaced → expires) is a single row, plotted on a `−90d → +30d` window with a `today` line. Marks at issuance, scheduled renewal, expiry, replacement.
- **Hover reveals ACME context.** Order URL, retries, challenge type — the operator language for cert debugging. Inline below the Gantt is the ACME event log with the same fixtures.
- **Tabs are unweighted, not pills.** Underlined active tab. The brief asked us to avoid Material — pill tabs read marketing-y.
- **Header is one row.** Hostname (mono), three pills (health, cert, version), four action buttons. No hero panel.

### Drafts (parity)
- Same diff component as the wizard review step, run over the staged batch. Stage / Apply / Discard at the page level; per-draft `…` menu lets you discard or promote one at a time. Order-aware apply ribbon at top when dependencies exist.

### Cmd-K palette
- **Scoped via prefix.** `>` actions, `#` hosts, `!` certs. Scope renders as a small chip in the input so the user always knows where they're searching.
- **Recents when empty.** No empty-state explainer text — the recent list is the explainer.
- **One keystroke targets.** Top actions show their own kbd shortcut. Add host is `A`, tail logs is `L`, stage & apply is `⇧↵`.

## Motion spec (the brief asked)

We override Radix's transition durations on three things:

| What | Old | New | Reason |
| --- | --- | --- | --- |
| Sheet enter | 200ms cubic-bezier | 220ms cubic-bezier(.2,.7,.3,1) | Translate-then-fade reads as "panel docking", not "modal popping". |
| Dialog enter | 150ms ease-out | 180ms cubic-bezier(.2,.7,.3,1) with a 3% scale | Pop-in feels intentional without bouncing. |
| Cmd-K enter | 100ms ease | 160ms cubic-bezier(.2,.7,.3,1) | The palette is a destination; let it land. |

Everything else keeps Radix defaults. All animations honor `prefers-reduced-motion: reduce` — they snap, no slide, no shimmer.

## Accessibility notes

- **Focus rings** are 2 px solid `--ring` with a 2 px offset — visible on both surfaces, both themes. Never `outline:none`.
- **Hit targets:** 28 px buttons, 30 px inputs, 36 px table rows. Bulk-action bar buttons are 24 px but spaced 6 px apart with 6 px padding (effective ~44 px touch area on touch devices).
- **Contrast:** `--fg` on `--bg` is 17.2:1 dark, 19.8:1 light. `--muted` is 5.4:1 / 5.7:1 — passes AA at 12 px+.
- **Sparkline** has `aria-label="last five status buckets"`. The 5xx column on the bulk-action bar is labelled, not just colored.

## What I'd push back on (brief invited it)

1. **"Sidebar must scale to 5+ tenants without becoming a scroll bar."** A tenant switcher at the top of the sidebar is the right pattern, but I'd argue tenants don't belong **as** the sidebar — they're a property the rest of the IA hangs off. Treating the workspace switch as a 44 px row at the very top frees the sidebar to scale to 30+ services without scrollbar pressure. The mock shows this.
2. **"Live logs as a streaming table."** Operators tail logs in a terminal. A streaming table fights the muscle memory. We kept the table but the per-request inspector drawer is the place to invest — request waterfall with Pingora-phase Server-Timing breakdown is where the product wins. Worth giving the inspector its own keyboard mode (`j/k` to walk rows, `/` to filter).
3. **"Gantt for cert lifecycle."** Lovely on paper, but with 500 hosts you need a *list* view of certs by days-to-expiry that lets you jump into one host's Gantt. The detail view is where the Gantt shines; the global Certificates page should be a sorted table with a Gantt-on-hover, not a Gantt-per-row.
4. **No filter panel on the Hosts list.** Brief implies it. Filter chips + URL query (`?q=` / `?status=`) beat a panel for keyboard-first ops — three keystrokes vs a click-tree.

## Open questions for engineering

- **Density toggle scope** — global, per-table, or per-tenant? The prototype is global.
- **Cmd-K scope chip on focus** — does pressing `⇥` cycle through scopes, or is it always typed (`>`)? Prototype assumes typed.
- **Wizard format default for edit** — dialog or sheet? Prototype shows dialog for edit, but `Cmd-click` to open inline would be ideal — let me know.

## Deliverable mapping

| Brief item | File |
| --- | --- |
| 1. Design rationale | `redesign-rationale.md` (this file) |
| 2. Tokens (Tailwind config snippet) | `tokens.md` |
| 3. Component-level redlines | `index.html` (interactive prototype) |
| 4. Empty states | Hosts list `state` tweak → `empty` |
| 5. Loading + error | Hosts list `state` tweak → `loading`, `error` |
| 6. Motion spec | Above |

Open `index.html`, flip Tweaks (toolbar button) for the full toolkit: theme, accent, density, sidebar, surface, hosts state, wizard format, diff style.
