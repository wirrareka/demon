// Shared primitive components + icons.
// All icons are inline SVG, single stroke, 24x24 viewBox, ~1.6 stroke-width —
// matching the lucide-react style the brief specifies. Drawn by hand here so
// the prototype has no dep, but in the real app these come from lucide-react.

const Icon = ({ d, size = 14, fill = 'none', stroke = 'currentColor', strokeWidth = 1.7, children, viewBox = '0 0 24 24', style }) => (
  <svg viewBox={viewBox} width={size} height={size} fill={fill} stroke={stroke}
       strokeWidth={strokeWidth} strokeLinecap="round" strokeLinejoin="round" style={style}>
    {d ? <path d={d} /> : children}
  </svg>
);

const I = {
  search:   (p) => <Icon {...p}><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></Icon>,
  plus:     (p) => <Icon d="M12 5v14M5 12h14" {...p}/>,
  chev:     (p) => <Icon d="m6 9 6 6 6-6" {...p}/>,
  chevR:    (p) => <Icon d="m9 6 6 6-6 6" {...p}/>,
  chevL:    (p) => <Icon d="m15 6-6 6 6 6" {...p}/>,
  check:    (p) => <Icon d="M5 12l4.5 4.5L19 7.5" {...p}/>,
  x:        (p) => <Icon d="M6 6l12 12M18 6l-12 12" {...p}/>,
  dots:     (p) => <Icon {...p}><circle cx="5" cy="12" r="1.4"/><circle cx="12" cy="12" r="1.4"/><circle cx="19" cy="12" r="1.4"/></Icon>,
  lock:     (p) => <Icon {...p}><rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></Icon>,
  globe:    (p) => <Icon {...p}><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/></Icon>,
  server:   (p) => <Icon {...p}><rect x="3" y="4" width="18" height="7" rx="1.5"/><rect x="3" y="13" width="18" height="7" rx="1.5"/><path d="M7 7.5h.01M7 16.5h.01"/></Icon>,
  shield:   (p) => <Icon d="M12 3l8 3v5c0 5-3.4 8.7-8 10-4.6-1.3-8-5-8-10V6z" {...p}/>,
  cert:     (p) => <Icon {...p}><rect x="3" y="4" width="18" height="14" rx="2"/><path d="M8 9h8M8 12h5M9 21v-3M15 21v-3"/></Icon>,
  list:     (p) => <Icon d="M4 7h16M4 12h16M4 17h10" {...p}/>,
  layers:   (p) => <Icon {...p}><path d="m3 7 9-4 9 4-9 4z"/><path d="m3 12 9 4 9-4M3 17l9 4 9-4"/></Icon>,
  bolt:     (p) => <Icon d="M13 2 4 14h6l-1 8 9-12h-6z" {...p}/>,
  pulse:    (p) => <Icon d="M3 12h4l3-7 4 14 3-7h4" {...p}/>,
  book:     (p) => <Icon {...p}><path d="M4 4v16a2 2 0 0 0 2 2h14V4H6a2 2 0 0 0-2 2z"/><path d="M4 18h16"/></Icon>,
  settings: (p) => <Icon {...p}><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/></Icon>,
  inbox:    (p) => <Icon {...p}><path d="M22 12h-6l-2 3h-4l-2-3H2"/><path d="M5.4 4.6a2 2 0 0 1 1.8-1.2h9.6a2 2 0 0 1 1.8 1.2L22 12v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-6z"/></Icon>,
  bell:     (p) => <Icon {...p}><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></Icon>,
  user:     (p) => <Icon {...p}><circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/></Icon>,
  sun:      (p) => <Icon {...p}><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></Icon>,
  moon:     (p) => <Icon d="M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z" {...p}/>,
  side:     (p) => <Icon {...p}><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/></Icon>,
  filter:   (p) => <Icon d="M3 5h18l-7 9v6l-4-2v-4z" {...p}/>,
  refresh:  (p) => <Icon {...p}><path d="M21 12a9 9 0 1 1-3-6.7L21 8"/><path d="M21 3v5h-5"/></Icon>,
  download: (p) => <Icon d="M12 3v12m0 0 4-4m-4 4-4-4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" {...p}/>,
  trash:    (p) => <Icon {...p}><path d="M3 6h18"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/></Icon>,
  copy:     (p) => <Icon {...p}><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></Icon>,
  edit:     (p) => <Icon d="M11 4H5a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2h13a2 2 0 0 0 2-2v-6M18.5 2.5a2.1 2.1 0 0 1 3 3L12 15l-4 1 1-4z" {...p}/>,
  alert:    (p) => <Icon {...p}><circle cx="12" cy="12" r="9"/><path d="M12 8v4M12 16h.01"/></Icon>,
  info:     (p) => <Icon {...p}><circle cx="12" cy="12" r="9"/><path d="M12 8h.01M11 12h1v4h1"/></Icon>,
  arrowR:   (p) => <Icon d="M5 12h14m-6-6 6 6-6 6" {...p}/>,
  cmd:      (p) => <Icon {...p}><path d="M6 6h3v3H6a2 2 0 1 1 0-4z"/><path d="M9 6h6v3H9z"/><path d="M15 6h3a2 2 0 1 1 0 4h-3z"/><path d="M9 9v6"/><path d="M15 9v6"/><path d="M6 15h3v3a2 2 0 1 1-4 0z"/><path d="M9 15h6v3H9z"/><path d="M15 15h3a2 2 0 1 1 0 4z"/></Icon>,
  enter:    (p) => <Icon d="M9 10h7a2 2 0 0 1 0 4H5m4-4-4 4m4 4-4-4" {...p}/>,
  power:    (p) => <Icon d="M12 3v10M5.6 6.6a8 8 0 1 0 12.8 0" {...p}/>,
  history:  (p) => <Icon {...p}><path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l3 2"/></Icon>,
  branch:   (p) => <Icon {...p}><circle cx="6" cy="5" r="2"/><circle cx="6" cy="19" r="2"/><circle cx="18" cy="7" r="2"/><path d="M6 7v10"/><path d="M18 9a4 4 0 0 1-4 4H8"/></Icon>,
  drafts:   (p) => <Icon {...p}><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6M9 13h6M9 17h4"/></Icon>,
};

window.I = I;
window.Icon = Icon;

// ── Pill / Chip / Kbd ─────────────────────────────────────────────────────
function Pill({ kind = 'default', children, dot }) {
  const cls = kind === 'default' ? 'pill' : `pill ${kind}`;
  return (
    <span className={cls}>
      {dot && <span className="dot" />}
      {children}
    </span>
  );
}

function Kbd({ children }) { return <span className="kbd">{children}</span>; }

function CertPill({ status, days }) {
  if (status === 'active')   return <Pill kind="success" dot>Active · {days}d</Pill>;
  if (status === 'renewing') return <Pill kind="warning" dot>Renewing · {days}d</Pill>;
  if (status === 'expiring') return <Pill kind="warning" dot>Expiring · {days}d</Pill>;
  if (status === 'expired')  return <Pill kind="danger"  dot>Expired · {Math.abs(days)}d</Pill>;
  if (status === 'pending')  return <Pill kind="info"    dot>Pending</Pill>;
  return <Pill>{status}</Pill>;
}

// Sparkline: 5 mini stacked-bars, color reflects worst class in that bucket.
function Sparkline({ codes }) {
  return (
    <span className="spark" aria-label="last five status buckets">
      {codes.map((b, i) => {
        const total = b[0] + b[1] + b[2] + b[3];
        if (total === 0) return <span key={i} className="b idle" />;
        const cls = b[3] > 5 ? 'bad' : b[2] > 10 ? 'warn' : '';
        const h = Math.max(4, Math.round((b[0] / 100) * 18));
        return <span key={i} className={`b ${cls}`} style={{ height: h }} />;
      })}
    </span>
  );
}

window.Pill = Pill;
window.Kbd = Kbd;
window.CertPill = CertPill;
window.Sparkline = Sparkline;
