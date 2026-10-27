# Button

Buttons start actions; the label says exactly what happens.

| Variant | Use when |
|---|---|
| primary | The one main action in a view or dialog. Ink fill on surface. |
| secondary | Supporting actions. Surface fill, `line` border. |
| ghost | Toolbars, dismiss, low emphasis. No border until hover. |
| danger | The destructive choice inside an inline confirmation. `bad` text. |

Sizes: `sm` 28px (`radius-md`, 12px text) and `md` 34px (`radius-lg`, 13px text). States: hover darkens the border to `ink-3` (primary drops to 90% opacity), focus-visible shows a 2px `focus` ring, disabled is 50% with a not-allowed cursor, loading swaps the icon for a spinner and keeps the label.

Do: one primary per area; verbs that name the effect ("Install", "Export for invoice"). Don't: two primaries side by side; "OK", "Submit"; colour a button with an account colour.

Accessibility: native `<button>`; icon-only buttons need an accessible name.
