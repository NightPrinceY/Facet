# Facet design system v1.0

Tokens: `tokens.css` (source) and `tokens.json` (generated). Reference build: `prototype/facet.html`. Design check: `/design-check`. Reviewer agent: `design-reviewer`.

---

## 1. Audit of prototype v4 (2026-10-25)

**Summary:** 22 component types reviewed · 7 issue groups · score **68/100** before this system, target 90+ in the app.

### Token coverage
| Category | Defined in prototype | Hardcoded found | Action |
|---|---|---|---|
| Colour | 29 tokens | 4 values (macOS window buttons, `#fff` initials) | Window buttons stay literal (OS convention). Initials move to `--on-facet-N`. |
| Spacing | none | 23 distinct px values | Use the 13-step `--space-*` scale. |
| Font size | none | 20 distinct sizes (10–30px) | Use the 10-step `--text-*` scale. |
| Radius | none | 17 distinct radii | Use 7 `--radius-*` steps. |
| Inline styles | — | 154 `style=""` in templates | Replace with component classes in the app. |

### Contrast (WCAG AA needs 4.5:1 for text under 18px)
| Token | Before | Ratio | After | Ratio |
|---|---|---|---|---|
| `--ink-3` light | #868E9A | 3.31 ✗ | #6B7380 | 4.78 ✓ |
| `--ink-3` dark | #69717E | 3.78 ✗ | #858D99 | 5.55 ✓ |
| `--ok` light | #0E9F6E | 3.39 ✗ | #067A53 | 5.36 ✓ (4.68 on `--ok-soft`; was #08875C, 3.96 on soft, D-014) |
| `--warn` light | #B76E00 | 4.00 ✗ | #9A5C00 | 5.38 ✓ |
| `--bad` light | #D92D45 | 4.76 ✓ | #C2263C | 5.78 ✓ (margin on tinted fills) |
| White initials on facets | — | 1.5–4.1 ✗ (except violet 5.1) | `--on-facet-N` = ink | 4.6–12.6 ✓ |

### Naming
| Issue | Recommendation |
|---|---|
| `f1…f6`, `fc`, `pc` mixed | `--facet-1…6`; components take `--account-color` |
| `.btn.pri` vs `.pill.ok` vs `.tag.pro` | BEM-ish: `btn btn--primary`, `pill pill--ok`, `tag tag--pro` in the app |
| `surface2`, `line2` | `--surface-2`, `--line-2` |

### Priority actions
1. Ship tokens.css as the only source of values; lint for raw px/hex in the app (stylelint `declaration-strict-value`).
2. Fix contrast tokens (done in tokens.css).
3. Build the 10 core components in section 4 as React components with stories before any view.

---

## 2. Principles
1. **Identity everywhere.** The active account's colour is visible wherever the user can type, act or read costs: rail marker, window tint, avatar, lane, chart series.
2. **Show before change.** Anything that writes to disk shows a diff or a preview first, and says a backup is made.
3. **Numbers you can trust.** Tabular figures, units always shown, estimates labelled "API-equivalent", sample data labelled.
4. **Quiet chrome, loud data.** Neutrals carry the UI; colour is reserved for accounts and states.
5. **Every state designed.** Default, hover, focus-visible, active, disabled, loading, empty, error.
6. **Native on each OS, one product.** Platform chrome adapts; layout and components do not.

---

## 3. Foundations

### Colour roles
| Role | Tokens | Use |
|---|---|---|
| Canvas | `--bg`, `--surface`, `--surface-2`, `--sunk`, `--rail` | Background layers, from page to wells |
| Lines | `--line`, `--line-2` | Borders, then dividers inside cards |
| Text | `--ink`, `--ink-2`, `--ink-3` | Primary, secondary, hint |
| Interaction | `--focus`, `--link` | Focus rings, text links |
| State | `--ok`, `--warn`, `--bad` + `-soft` | Status pills, callouts, meters over threshold |
| Accounts | `--facet-1…6`, `--on-facet-1…6` | Account identity only. Never for state. |
| Data viz | `--viz-*` | Token types; accounts reuse facets |

Primary actions use `--ink` on `--surface` (inverted button), so account colours never compete with the main action.

### Typography
| Token | Size | Use | Face |
|---|---|---|---|
| `--text-4xl` | 36 | Marketing headline | Display |
| `--text-3xl` | 28 | Brand wordmark | Display 650 |
| `--text-2xl` | 24 | View titles, stat values | Display 650, tracking −0.03em |
| `--text-xl` | 18 | Modal and drawer titles | Display 650 |
| `--text-lg` | 16 | Card titles | Body 600 |
| `--text-md` | 14 | Body | Body 400 |
| `--text-base` | 13 | Controls, tables, lists | Body 400/500 |
| `--text-sm` | 12 | Hints, meta, pills | Body 400/500 |
| `--text-xs` | 11 | Eyebrows (uppercase, +0.06em), tags, kbd | Body 500/600 |
| `--text-2xs` | 10.5 | Chart ticks only | Body |

Mono (`--font-mono`) for paths, commands, ids, timestamps. Arabic uses `--font-arabic`; display tracking resets to 0 in RTL.

### Space, radius, elevation, motion
- Space: 4px base, steps 2·4·6·8·12·16·20·24·32·40·48·64. Card padding 16 (`--space-6`); view padding 26/30 → use 24/32.
- Radius: 4 ticks · 6 code · 8 small controls · 10 controls · 14 cards · 16 window/modal · full pills.
- Elevation: 1 buttons, 2 menus/toasts/popovers, 3 window/modal/drawer.
- Motion: 120 ms hovers, 200 ms toggles and bars, 320 ms drawers; `--ease-out`. All motion off under reduced motion.

---

## 4. Components

Each component lists variants, sizes, states, accessibility, and do/don't. Props are for the React build.

### Button
| Variant | Use when |
|---|---|
| `primary` | The main action of a view or dialog (one per area) |
| `secondary` | Supporting actions |
| `ghost` | Toolbars, dismiss, low-emphasis |
| `danger` | Destructive confirm inside an inline confirmation |
Props: `variant`, `size: sm|md`, `icon`, `loading`, `disabled`. States: hover (border `--ink-3`; primary 90% opacity), focus-visible ring 2px `--focus`, disabled 50% + `not-allowed`, loading (spinner replaces icon, label stays). A11y: native `<button>`; icon-only needs `aria-label`. Don't: two primary buttons side by side; vague labels ("OK").

### IconButton
34×34 (`--control-height`), radius 10, border `--line`. Always `aria-label`. Used for theme, zoom, close.

### Segmented control
Group of 2–5 mutually exclusive options (range 24h/7d/30d, platform, zoom presets). `role="group"`, buttons with `aria-pressed`. Selected: `--surface` with shadow-1 on `--sunk` track. Don't use for actions.

### Toggle (switch)
34×20. `role="switch"` + `aria-checked`. On = `--ink`. Always paired with a visible label and one-line description in a Settings row. Mirrors in RTL.

### Pill and Tag
Pill: status and metadata (`neutral`, `ok`, `warn`, `bad`, `example` dashed). Tag: tiny uppercase labels (`LAB`, `PRO`). Pro tag is inverted ink. Don't use pills as buttons unless they are filter chips with `aria-pressed`.

### Account avatar
Rounded square with the account initial. Sizes 26 (lists), 40 (rail), 42 (cards, scope header). Background `--facet-N`, text `--on-facet-N`. Running dot (12px, `--ok`, ring in the parent background). A11y: decorative inside a labelled row; standalone needs `aria-label` with the account name.

### Account rail
Vertical list of avatars (horizontal on narrow screens). First item "All accounts", last "Add account". Selected: 4px ink marker on the inline-start edge, radius morphs 12→15 on hover. `aria-pressed` per item. Selecting scopes every view and tints the window.

### Window tint
2px inset ring in the active account's colour around the app window plus a colour dot in the title. Off per account in Accounts. Purpose: you always know which account you are in.

### Nav item
Icon + label + optional count/tag. Current: `aria-current="page"`, surface fill and border. Counts in `--ink-3`; alerts in `--bad`.

### Card, Stat, List row, Table
- Card: `--surface`, border `--line`, radius 14. Use for groups, not for every block.
- Stat: eyebrow label, value in display 24, one-line context. Use only when the number is the point (Today, Costs).
- List row: grid with avatar/icon, text block (title + hint), trailing action. Dividers `--line-2`.
- Table: header `--surface-2`, uppercase 11px labels; wrap in a scroll container on narrow screens.

### Meter
Label, value, 6px bar in the account colour; turns `--bad` above the alert threshold (default 80%). Values tabular. `role="meter"` with `aria-valuenow/min/max` in the app.

### Input, Select, Textarea
Height 34, radius 10, border `--line`; focus border `--focus` + 3px focus halo. Labels above (12px, `--ink-2`), hint below (12px, `--ink-3`), error below in `--bad` with an icon and how to fix it.

### Callout
Inline message with optional icon and action: `neutral`, `warn`, `bad`. Says what is wrong and what to do. Never for success that needs no action (use a toast).

### Toast
Bottom inline-end, ink background, 3.8 s, max 400px, `aria-live="polite"`. One sentence, past tense ("Saved for Work · backup kept").

### Modal with stepper
For multi-step flows (Add account: Name → Create → Sign in → Check). Steps show done (✓, `--ok`), current (ink), upcoming. Continue disabled until the step is valid. `role="dialog"`, `aria-modal`, focus trapped, Esc closes (except mid-sign-in, which asks first).

### Drawer
Inline-end panel, 480px, for previews (Ecosystem safety preview). Header with identity, scrollable body, sticky footer with the decision. Opens with 320 ms slide; Esc closes.

### Menu
Popover with ≤ 6 actions, destructive last in `--bad`. Arrow keys move, Esc closes, focus returns to trigger.

### Command palette
Ctrl/⌘ K. Fuzzy filter over accounts and actions. Arrow keys + Enter. Shows account avatars beside account actions.

### Drill row and breadcrumbs
Costs drill-down: name + sub, bar scaled to the max in view, tokens, cost. Clickable rows are buttons; leaf rows are not. Breadcrumb links in `--link`, last item plain.

### Timeline (canvas)
Lanes per account (40px), spend histogram (54px), axis (26px), minimap (34px). Events: prompt = 2px ink bar; model call = dot sized by √cost in the account colour with a duration tail; tool run = 6px square (`--ink-3`, failed `--bad`) with tail. Selected = 2px ink ring. Now line ink; replay playhead `--warn`. Keyboard: ←/→ step, +/− zoom. Canvas has `aria-label` and the inspector mirrors the selection as text for screen readers. Spec: `docs/02-product/TIMELINE-SPEC.md`.

### Risk meter and safety-preview items
Risk: short bar filled ⅓/⅔/full in `--ok`/`--warn`/`--bad` + text "Low/Medium/High risk" (never colour alone). Preview item: 22px badge (`!` hook, `$` env, `↗` network, `▶` start command, `◧` files, `✓` permission) + plain sentence + code chip.

### Locked (Pro) veil
Blurred overlay over a Pro section with lock icon, one sentence, "Free covers one account at a time", and "Preview Pro". Never hide that a feature exists.

### Platform chrome
Linux (GNOME): centred title, round controls at inline-end. Windows: title at start, 44px controls flush end. macOS: traffic lights at start, centred title. Window radius 16 in the prototype; the app uses native decorations where available.

---

## 5. Patterns

### Account identity
Rail marker + window tint + title dot + avatar in every row that belongs to an account + lane and chart colour. Colour order for new accounts: first unused facet.

### Show before change
Preview (diff, file list, safety preview) → explicit button that names the effect ("Review and save", "Install") → toast with what happened and that a backup exists → undo path in Health › Backups.

### Guided sign-in
Explain why other accounts close → one action to start → waiting state with live status → success → verify email. Never auto-type credentials.

### Empty, loading, error
- Empty: say what will appear and how to make it appear ("No Claude Code activity yet. Open a terminal as Work to start.").
- Loading: skeleton rows for lists; spinner only for single checks (Health).
- Error: what happened, why if known, the fix, and a retry.

### Pro gating
Free shows the feature for one account; cross-account parts show the veil. Plan switch in Settings previews both.

### Right-to-left
Logical properties only (`inline-start/end`, `padding-inline`). Mirror direction icons (arrows, drawers); never mirror clocks, charts' time axis or code. Arabic font and no negative tracking.

### Data labels
Tokens as K/M with one decimal; money as $ with 2 decimals (3–4 under $0.01); times `HH:MM` coarse, `HH:MM:SS.mmm` in the inspector; durations `ms / s / m s / h m`. Always say "API-equivalent" for cost and "sample data" for examples.

---

## 6. Content guidelines
- Name things the way users do: accounts, costs, settings, plugins. Not "profiles", "telemetry ingestion", "config".
- Buttons say what happens: "Launch", "Review and save", "Install", "Export for invoice".
- Errors explain and fix: "Port 4318 is in use. Facet picked 4319 and will update each account's settings after you approve."
- No apologies, no exclamation marks, no jokes in errors.
- Compliance copy is plain and specific ("Facet never reads or copies sign-in data").
