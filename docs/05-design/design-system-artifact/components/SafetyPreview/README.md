# SafetyPreview

The plain-language list of everything a skill, plugin or MCP server would add, shown before anything is installed.

Anatomy: a risk line (short bar filled a third, two thirds or fully in `ok`, `warn` or `bad`, always with the words "Low/Medium/High risk" and the reasons), then one row per effect: a 22px badge and a sentence with a code chip. Badges: `!` hook, `$` environment variable, `↗` network, `▶` start command, `◧` files written, `✓` permission. Warning badges use `warn-soft`, dangerous ones `bad-soft`.

Lives in the Ecosystem drawer above the account checkboxes and the Install button. Unpinned start commands (`@latest`, `npx -y`) are always flagged.
