# Meter

A labelled bar for a share of a limit or budget, in the account's colour until it crosses the alert threshold.

Anatomy: label and value on one line (12px, tabular figures), a 6px bar on `sunk`, fill in `facet-N`. At or above the threshold (default 80%) the fill turns `bad`. The value always includes the unit or the reset time: "71% · resets in 1h 12m".

Use for the 5-hour session window, the weekly limit and client budgets. Don't use for progress of a running task (use a spinner or progress text).

Accessibility: `role="meter"` with `aria-valuenow`, `aria-valuemin`, `aria-valuemax` and a label in the app.
