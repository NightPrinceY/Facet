# AccountAvatar

The account's initial on its facet colour, the smallest unit of account identity.

Sizes: 26px (`radius-md`) in rows and pills, 40px in the rail, 42px (`radius-lg`) on cards and the scope header. Fill `facet-N`, initial in `on-facet-N` (ink, except white on `facet-2` in light) using the display face at 650. A 12px `ok` dot with a ring in the parent background marks a running account.

Do: put an avatar on every row, pill, lane and chart legend that belongs to an account. Don't: use facet colours for anything that is not an account; put white initials on facets other than `facet-2` in light, which fails contrast.

Accessibility: decorative inside a labelled row; on its own it needs the account name as its accessible name.
