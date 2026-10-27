# Toggle

A switch for a setting that takes effect immediately, always inside a settings row with a label and a one-line description.

34 by 20px, `radius-full`. Off: `line` track. On: `ink` track, knob slides 14px (mirrored in right-to-left). 200ms ease-out.

Don't use a toggle for something that needs a save step; use a checkbox in a form instead.

Accessibility: `role="switch"` with `aria-checked` and an accessible name matching the row label.
