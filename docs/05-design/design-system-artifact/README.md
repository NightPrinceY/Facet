# Facet

Facet keeps a person's Claude accounts apart, shows what each one costs to the second, gives full control of every setting, and previews what a plugin will do before it is installed. This is its design system: the rules and pieces every screen is built from. Facet is an unofficial tool, not affiliated with or endorsed by Anthropic.

## Principles

1. **Identity everywhere.** The active account's colour is visible wherever the user can type, act or read a cost: the rail marker, a 2px window tint, the avatar on every row, the Timeline lane and the chart series.
2. **Show before change.** Anything that writes to disk shows a diff or preview first, names its effect on the button ("Review and save", "Install"), and says a backup was kept.
3. **Numbers you can trust.** Tabular figures, units always shown, costs labelled "API-equivalent", examples labelled "sample data".
4. **Quiet chrome, loud data.** Neutrals carry the interface. Colour belongs to accounts and states, nothing else.
5. **Every state designed.** Default, hover, focus-visible, active, disabled, loading, empty, error.
6. **Native on each OS, one product.** Window chrome follows Linux, Windows or macOS; layout and components do not change.

## Voice

Plain words, the user's words. People manage *accounts*, *costs*, *settings* and *plugins*, never "profiles", "telemetry" or "config". Buttons say what happens. Errors say what went wrong and how to fix it, with no apology. Compliance copy is specific: "Facet never reads or copies sign-in data."

| Write | Not |
|---|---|
| Review and save | Submit |
| Saved for Work · backup kept | Success! |
| Port 4318 is in use. Facet picked 4319 and will update each account after you approve. | Error: EADDRINUSE |
| Sample data | Lorem ipsum |

## Colour

Neutral first. `ink` on `surface` is the primary action (an inverted button), so account colours never compete with the main action.

- **Canvas:** `bg` (page) → `surface` (cards) → `surface-2` (nav, headers) → `sunk` (wells, tracks) → `rail` (account rail).
- **Lines:** `line` for borders, `line-2` for dividers inside a card.
- **Text:** `ink` primary, `ink-2` secondary, `ink-3` hints. All meet 4.5:1 on `surface` in both themes.
- **State:** `ok`, `warn`, `bad`, each with a `-soft` background. State is never shown by colour alone: pair it with a word or icon.
- **Accounts:** `facet-1` to `facet-6`, assigned in order to new accounts. Initials sit on them in `on-facet-N`, which is ink everywhere except `facet-2` in light (white). White on most facets fails contrast.

Light is the first theme; dark mirrors every token. Dark surfaces are near-black with a slight blue bias, not grey.

## Type

- **Bricolage Grotesque** (display, weight 650, tracking −0.03em) for the wordmark, view titles, stat values and dialog titles. Used with restraint.
- **Geist** for everything else, 11–16px. Eyebrows are 11px uppercase with +0.06em tracking.
- **Geist Mono** for paths, commands, ids and timestamps. The Timeline inspector shows time as `14:21:57.983`.
- **IBM Plex Sans Arabic** in right-to-left mode; display tracking resets to 0.

All three Latin faces load from Google Fonts. Numbers in tables, meters and stats use tabular figures.

## Space, shape, depth, motion

- 4px grid: `space-1` (2px) to `space-12` (64px). Card padding `space-6`, section gaps `space-8`, view sides `space-9`.
- Radius by role: 4 ticks · 6 code · 8 small controls · 10 controls · 14 cards · 16 window and dialogs · full pills.
- Three shadows: `shadow-1` buttons, `shadow-2` menus and toasts, `shadow-3` window, modals, drawers.
- Motion: 120ms hovers, 200ms toggles and bars, 320ms drawers, ease-out `cubic-bezier(.2,.8,.2,1)`. All motion stops under reduced motion.

## Iconography

18px grid, 1.6px round strokes, no fills except small dots. Icons sit beside a label; icon-only buttons always carry an accessible name. Direction icons (arrows, drawer edges) mirror in right-to-left; clocks, time axes and code never do. There is no logo file yet: the mark is a hexagon split into four facet-coloured triangles, drawn in the app.

## Layout

The app window is: account rail (64px) · section nav (212px) · workspace. On narrow screens the rail and nav become horizontal strips above the workspace. Selecting an account in the rail scopes every view to it and tints the window border in its colour.

## Patterns

- **Show before change:** preview → named action → toast with what happened and that a backup exists → undo in Health.
- **Guided sign-in:** explain why other accounts close for a moment → start → live waiting state → success → confirm the email.
- **Pro gating:** the section stays visible behind a blurred veil with a lock, one sentence and "Preview Pro". Never hide that a feature exists.
- **Empty states** say what will appear and how to make it appear. **Errors** say what happened, why if known, and the fix.
- **Right-to-left:** logical properties only (`inline-start`, `padding-inline`).

## Components

Button · AccountAvatar · AccountRail · Pill · Meter · Toggle · Callout · SafetyPreview · TimelineLane. Each has a guideline and a live preview under Components. The full catalogue for the app, including modals, drawers, the command palette and the Timeline canvas, is in the project repository at `docs/05-design/DESIGN-SYSTEM.md`.
