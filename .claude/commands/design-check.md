---
description: Check UI work against the Facet design system
---

Review the UI change in $ARGUMENTS (or the current diff) against `docs/05-design/DESIGN-SYSTEM.md` and `docs/05-design/tokens.css`.

Report, most important first:
- Raw colours, font sizes, spacing, radii or shadows that should be tokens.
- Components used differently from their spec (variants, states, sizes).
- Missing states: hover, focus-visible, disabled, loading, empty, error.
- Account identity: is the active account's colour visible wherever the user can type or act?
- Accessibility: contrast in light and dark, keyboard path, ARIA roles, reduced motion.
- RTL: logical properties (`inline-start/end`), mirrored icons where direction matters.
- Copy: plain words, active voice, buttons that say what happens.

Give each finding a file and line, and a concrete fix.
