// Cmd-K palette. Scoped search via prefix (>actions, #hosts, !certs).
// Up/Down nav, Enter to fire, Esc to close. Recents at top when empty query.

function CmdK({ open, onClose, onJumpTo, onAddHost }) {
  const [q, setQ] = React.useState('');
  const [idx, setIdx] = React.useState(0);

  // Build the items list dynamically based on query/scope.
  const items = React.useMemo(() => {
    const out = [];
    const query = q.trim();
    let scope = null;
    let needle = query;
    if (query.startsWith('>')) { scope = 'actions'; needle = query.slice(1).trim(); }
    else if (query.startsWith('#')) { scope = 'hosts'; needle = query.slice(1).trim(); }
    else if (query.startsWith('!')) { scope = 'certs'; needle = query.slice(1).trim(); }
    const match = (s) => !needle || s.toLowerCase().includes(needle.toLowerCase());

    if (!query) {
      out.push({ kind: 'section', label: 'Recents' });
      window.CMDK.recents.forEach(r => out.push({ ...r }));
    }

    if (!scope || scope === 'actions') {
      const acts = window.CMDK.actions.filter(a => match(a.label));
      if (acts.length) {
        out.push({ kind: 'section', label: 'Actions' });
        acts.forEach(a => out.push(a));
      }
    }
    if (!scope || scope === 'hosts') {
      const hosts = window.HOSTS.filter(h => match(h.host)).slice(0, 6);
      if (hosts.length) {
        out.push({ kind: 'section', label: 'Hosts' });
        hosts.forEach(h => out.push({ kind: 'host', label: h.host, meta: h.up, raw: h }));
      }
    }
    if (!scope || scope === 'certs') {
      const certs = window.HOSTS.filter(h => match(h.host)).slice(0, 4);
      if (certs.length) {
        out.push({ kind: 'section', label: 'Certificates' });
        certs.forEach(h => out.push({ kind: 'cert', label: `cert · ${h.host}`, meta: `${h.certDays}d` }));
      }
    }
    return out;
  }, [q]);

  const navigable = items.map((it, i) => ({ it, i })).filter(x => x.it.kind !== 'section');

  React.useEffect(() => { setIdx(0); }, [q]);

  React.useEffect(() => {
    if (!open) return;
    const onKey = (e) => {
      if (e.key === 'Escape') { e.preventDefault(); onClose(); }
      else if (e.key === 'ArrowDown') {
        e.preventDefault();
        setIdx(i => Math.min(i + 1, navigable.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setIdx(i => Math.max(0, i - 1));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        const sel = navigable[idx]?.it;
        if (!sel) return;
        if (sel.kind === 'host')   { onJumpTo({ kind: 'host', raw: sel.raw }); onClose(); }
        else if (sel.kind === 'action') {
          if (sel.label.startsWith('Add host')) { onAddHost(); onClose(); }
          else onClose();
        } else onClose();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, idx, navigable, onClose, onJumpTo, onAddHost]);

  if (!open) return null;
  const navIdx = (origIdx) => navigable.findIndex(x => x.i === origIdx);

  return (
    <div className="cmdk-scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="cmdk-shell">
        <div className="cmdk-input">
          <I.search />
          {q.startsWith('>') && <span className="scope">actions</span>}
          {q.startsWith('#') && <span className="scope">hosts</span>}
          {q.startsWith('!') && <span className="scope">certs</span>}
          <input
            autoFocus
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search hosts, upstreams, certs · or > for actions, # for hosts, ! for certs"
          />
          <Kbd>esc</Kbd>
        </div>
        <div className="cmdk-list scroll">
          {items.map((it, i) => {
            if (it.kind === 'section') return <div key={i} className="cmdk-section">{it.label}</div>;
            const active = navIdx(i) === idx;
            const Icn = it.kind === 'action' ? I.bolt : it.kind === 'cert' ? I.cert : it.kind === 'host' ? I.globe : I.history;
            return (
              <button key={i} className={`cmdk-row ${active ? 'active' : ''}`}
                      onMouseEnter={() => setIdx(navIdx(i))}
                      onClick={() => {
                        if (it.kind === 'host')   { onJumpTo({ kind: 'host', raw: it.raw }); onClose(); }
                        else if (it.kind === 'action' && it.label.startsWith('Add host')) { onAddHost(); onClose(); }
                        else onClose();
                      }}>
                <Icn className="ic" />
                <span className="lbl mono" style={{ fontFamily: it.kind === 'action' ? 'Inter, sans-serif' : undefined }}>{it.label}</span>
                {it.meta && <span className="meta mono">{it.meta}</span>}
                {it.kbd && <span className="kbd-row"><Kbd>{it.kbd}</Kbd></span>}
                {active && <I.enter size={13} style={{ color: 'var(--muted)' }} />}
              </button>
            );
          })}
        </div>
        <div className="cmdk-foot">
          <span className="grp"><Kbd>↑</Kbd><Kbd>↓</Kbd> navigate</span>
          <span className="grp"><Kbd>↵</Kbd> open</span>
          <span className="grp"><Kbd>⇥</Kbd> scope</span>
          <span style={{ flex: 1 }} />
          <span className="grp">{navigable.length} results</span>
        </div>
      </div>
    </div>
  );
}

window.CmdK = CmdK;
