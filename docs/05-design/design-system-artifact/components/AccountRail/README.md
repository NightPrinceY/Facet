# AccountRail

The vertical strip of account avatars on the window's inline-start edge; choosing one scopes the whole app to that account.

Order: "All accounts" first, a divider, one 40px avatar per account, then a dashed "Add account" tile. The selected item shows a 4px ink marker on the inline-start edge. Hover rounds the tile from 12px to 15px. On narrow screens the rail runs horizontally and the marker moves under the tile.

Selecting an account also tints the window with a 2px inset ring in its colour and adds a colour dot to the title, unless the user turned tint off for that account.

Accessibility: each item is a button with `aria-pressed` and the account name as its label. Arrow keys move between items in the app.
