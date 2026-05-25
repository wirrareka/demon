# Contrast Verification — v0.5 Token Foundation

**Computed:** 2026-05-18 (DS1 reconciliation pass)
**Method:** WCAG 2.1 relative luminance (IEC 61966-2-1 linearisation → OKLCH → XYZ → Y).
All OKLCH values converted via the OKLab ↔ LMS matrix from the CSS Color 4 spec.
**Reference bg values used:** `oklch(0.165 0.008 250)` (dark) · `oklch(0.995 0.003 85)` (light).

---

## Dark theme — cheatsheet targets

| Pair | OKLCH value | Lum | Contrast | Target | Pass |
|---|---|---|---|---|---|
| `fg` on `bg` | `oklch(0.975 0.003 250)` | 0.9154 | **17.93:1** | ≥17.2 | ✓ AAA |
| `fg-2` on `bg` | `oklch(0.860 0.006 250)` | 0.6926 | **12.59:1** | ≥12.4 | ✓ AAA |
| `muted` on `bg` | `oklch(0.640 0.012 250)` | 0.3375 | **5.73:1** | ≥5.7 (AA ≥12px) | ✓ AA |
| `muted-2` on `bg` | `oklch(0.527 0.012 250)` | 0.2003 | **3.61:1** | ≥3.6 (AA ≥14px) | ✓ AA |
| `bg` | `oklch(0.165 0.008 250)` | 0.0045 | — | — | — |

> **DS1 fix — dark `muted-2`:** was `oklch(0.480 0.012 250)` → 2.95:1 (FAIL).
> Corrected to `oklch(0.527 0.012 250)` → 3.61:1 (PASS).
> The spec hex `#5a5a63` (tokens.md) itself gives only 2.92:1 on the live dark bg —
> the cheatsheet claimed 3.6:1 was achievable, so we solved for the OKLCH value that
> hits that ratio on the actual bg rather than shipping the spec hex literally.

### Status hues (dark, fg on `*-bg`)

| Pair | Contrast | Target | Pass |
|---|---|---|---|
| `success` on `success-bg` | **9.32:1** | ≥8.1 | ✓ AAA |
| `warning` on `warning-bg` | **10.31:1** | ≥8.6 | ✓ AAA |
| `danger` on `danger-bg` | **7.08:1** | ≥7.0 | ✓ AAA |

---

## Light theme — cheatsheet targets

| Pair | OKLCH value | Contrast | Target | Pass |
|---|---|---|---|---|
| `fg` on `bg` | `oklch(0.131 0.012 250)` | **19.81:1** | ≥19.8 | ✓ AAA |
| `fg-2` on `bg` | `oklch(0.320 0.012 250)` | **12.49:1** | ≥10.4 | ✓ AAA |
| `muted` on `bg` | `oklch(0.440 0.012 250)` | **7.65:1** | ≥5.4 (AA ≥12px) | ✓ AAA |
| `muted-2` on `bg` | `oklch(0.599 0.010 250)` | **3.90:1** | ≥3.9 (AA ≥14px) | ✓ AA |
| `bg` | `oklch(0.995 0.003 85)` | — | — | — |

> **DS1 fix — light `fg`:** was `oklch(0.185 0.012 250)` → 18.37:1 (below 19.8 target).
> styles.css also used 0.185, but both drifted from tokens.md hex `#09090b` (lum 0.00278).
> Corrected to `oklch(0.131 0.012 250)` → 19.81:1. The bg is near-white not pure-white,
> which is why the cheatsheet's 19.8 (calculated on `#ffffff`) requires a slightly darker fg.
>
> **DS1 fix — light `muted-2`:** was `oklch(0.660 0.010 250)` → 3.06:1 (FAIL).
> Corrected to `oklch(0.599 0.010 250)` → 3.90:1 (PASS).
> Same root cause: styles.css 0.660 doesn't hit 3.9:1 on the actual near-white bg.

---

## Spec vs styles.css discrepancies resolved

| Token | tokens.md hex | styles.css OKLCH | Live OKLCH (before) | Shipped OKLCH (after) | Resolution |
|---|---|---|---|---|---|
| dark `bg` | `#09090b` | `oklch(0.165 0.008 250)` | same | unchanged | styles.css kept — bg value passes all dependent ratios |
| dark `muted-2` | `#5a5a63` | `oklch(0.480 0.012 250)` | same | `oklch(0.527 0.012 250)` | Both spec values failed 3.6:1; corrected to meet cheatsheet guarantee |
| light `fg` | `#09090b` | `oklch(0.185 0.012 250)` | same | `oklch(0.131 0.012 250)` | styles.css drifted lighter; corrected to hit 19.8:1 on actual bg |
| light `muted-2` | `#a1a1aa` | `oklch(0.660 0.010 250)` | same | `oklch(0.599 0.010 250)` | styles.css too light on actual bg; corrected to meet 3.9:1 |

**Tokens not in this file (live-only extras, no spec conflict):**
`--row-hover`, `--breadcrumb-sep`, `--success-fg`, `--warning-fg`, `--danger-fg`, `--info-fg`,
`--info`, `--info-bg`, `--info-border` — all present in live `tokens.css` but absent from
`styles.css`. Kept as-is; they extend the spec without conflicting.

---

## Files touched

- `web/src/styles/tokens.css` — 3 token values changed (dark `muted-2`, light `fg`, light `muted-2`)
- `docs/design/v0.5/CONTRAST-VERIFIED.md` — this file
- `web/tailwind.config.ts` — no changes required (aliases correct, no values hardcoded)
