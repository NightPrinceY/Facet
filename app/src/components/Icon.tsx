// Line icons on an 18×18 grid, from the prototype (prototype/facet.html).

const PATHS = {
  today: <><rect x="3" y="4" width="12" height="11" rx="2" /><path d="M3 7.5h12M6.5 2.5v3M11.5 2.5v3" /></>,
  timeline: <><path d="M2.5 9h13" /><circle cx="5" cy="9" r="1.6" fill="currentColor" /><circle cx="10" cy="9" r="2.4" /><path d="M14 6v6M7.5 5v8" /></>,
  accounts: <><circle cx="6.5" cy="7" r="2.3" /><path d="M2.5 14c.5-2.2 2-3.3 4-3.3s3.5 1.1 4 3.3" /><circle cx="12.5" cy="6" r="1.8" /><path d="M11.5 9.6c1.9 0 3.4.9 4 3" /></>,
  costs: <path d="M3 14.5V9M7 14.5V5M11 14.5v-4M15 14.5V3.5" />,
  control: <><path d="M4 3v12M9 3v12M14 3v12" /><circle cx="4" cy="11" r="1.7" /><circle cx="9" cy="6" r="1.7" /><circle cx="14" cy="10" r="1.7" /></>,
  studio: <><path d="M3.5 14.5 10 8" /><path d="m11 3 1 2 2 1-2 1-1 2-1-2-2-1 2-1z" /><path d="m14.5 10.5.5 1 1 .5-1 .5-.5 1-.5-1-1-.5 1-.5z" /></>,
  ecosystem: <><path d="M9 2.5 15 6v6l-6 3.5L3 12V6z" /><path d="M3 6l6 3.5L15 6M9 9.5v6" /></>,
  sessions: <path d="M3 4h12v8H8l-3 2.5V12H3z" />,
  bridge: <path d="M2.5 12.5c2-4 4.5-6 6.5-6s4.5 2 6.5 6M2.5 12.5h13M6 12.5V9.3M12 12.5V9.3M9 12.5V6.5" />,
  health: <path d="M2.5 9h3l1.5-4 3 8 1.5-4h4" />,
  settings: <><circle cx="9" cy="9" r="2.3" /><path d="M9 2.5v2M9 13.5v2M2.5 9h2M13.5 9h2M4.4 4.4l1.4 1.4M12.2 12.2l1.4 1.4M4.4 13.6l1.4-1.4M12.2 5.8l1.4-1.4" /></>,
  plus: <path d="M9 4v10M4 9h10" />,
  play: <path d="M6 4.5v9l7-4.5z" fill="currentColor" stroke="none" />,
  stop: <rect x="5" y="5" width="8" height="8" rx="1.5" />,
  term: <><rect x="2.5" y="3.5" width="13" height="11" rx="2" /><path d="m5.5 7.5 2 1.5-2 1.5M9.5 11h3" /></>,
  grid: <><rect x="3" y="3" width="5" height="5" rx="1.2" /><rect x="10" y="3" width="5" height="5" rx="1.2" /><rect x="3" y="10" width="5" height="5" rx="1.2" /><rect x="10" y="10" width="5" height="5" rx="1.2" /></>,
  shield: <><path d="M9 2.5 14.5 4.5v4c0 3.3-2.3 5.8-5.5 7-3.2-1.2-5.5-3.7-5.5-7v-4z" /><path d="m6.5 9 1.8 1.8L11.8 7" /></>,
  check: <path d="m4 9.5 3.2 3L14 5.5" />,
  alert: <><path d="M9 2.5 16 15H2z" /><path d="M9 7.5v3.5M9 13v.01" /></>,
  info: <><circle cx="9" cy="9" r="6.5" /><path d="M9 8v4.5M9 5.5v.01" /></>,
  edit: <path d="m11.5 3.5 3 3-8 8H3.5v-3z" />,
  trash: <path d="M3.5 5h11M7 5V3.5h4V5M5 5l.8 9.5h6.4L13 5" />,
  launcher: <><rect x="3" y="3" width="12" height="12" rx="3" /><path d="M7 9h4M9 7v4" /></>,
  user: <><circle cx="9" cy="6.5" r="2.8" /><path d="M3.5 15c.8-3 3-4.5 5.5-4.5s4.7 1.5 5.5 4.5" /></>,
  window: <><rect x="2.5" y="3.5" width="13" height="11" rx="2" /><path d="M2.5 6.5h13" /><path d="M5 5h.01M7 5h.01" /></>,
  folder: <path d="M2.5 5.5c0-.8.6-1.5 1.5-1.5h3l1.5 1.5H14c.8 0 1.5.7 1.5 1.5v6c0 .8-.7 1.5-1.5 1.5H4c-.9 0-1.5-.7-1.5-1.5z" />,
  search: <><circle cx="8" cy="8" r="4.5" /><path d="m11.5 11.5 3 3" /></>,
  download: <path d="M9 3v8.5M5.5 8 9 11.5 12.5 8M3.5 14.5h11" />,
  refresh: <path d="M14.5 9a5.5 5.5 0 1 1-1.6-3.9M14.5 3.5v3h-3" />,
  earlier: <path d="M11 4.5 6.5 9l4.5 4.5" />,
  later: <path d="M7 4.5 11.5 9 7 13.5" />,
  image: <><rect x="2.5" y="3.5" width="13" height="11" rx="2" /><circle cx="6.5" cy="7.5" r="1.3" /><path d="m3 13 3.5-3.5 2.5 2.5 2-2 4 3.5" /></>,
  arrowBack: <path d="M14.5 9h-11M7.5 5 3.5 9l4 4" />,
} as const;

export type IconName = keyof typeof PATHS;

/** `mirror` flips direction icons (arrows, drawers) in right-to-left. */
export function Icon({ name, size = 16, mirror }: { name: IconName; size?: number; mirror?: boolean }) {
  return (
    <svg
      className={(mirror ?? (name === "earlier" || name === "later")) ? "icon--mirror" : undefined}
      width={size}
      height={size}
      viewBox="0 0 18 18"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {PATHS[name]}
    </svg>
  );
}
