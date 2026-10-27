# TimelineLane

One account's row on the Timeline canvas, showing tasks at coarse zoom and individual events at fine zoom.

Lane height 40px on `surface-2`, label column 110px with the avatar. Coarse zoom (over 45 minutes): task bars in the account colour from prompt to last event. Fine zoom: a 2px `ink` bar per prompt, a dot per model call sized by the square root of its cost with a faint duration tail, and a 6px square per tool run (`ink-3`, `bad` if failed). The selected event gets a 2px ink ring; the now line is ink and the replay playhead is `warn`.

The canvas is paired with a text inspector that mirrors the selection (time to the millisecond, tokens, cost) so the information is available to screen readers and keyboard users (left and right arrows step through events).
