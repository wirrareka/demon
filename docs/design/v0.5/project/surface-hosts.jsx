// Hosts list — the most-trafficked screen. Density-first, 36px rows compact,
// 30px extra-compact. Filter chips, search, sticky header, bulk-select bar.
// Empty/loading/error states gated by tweak.

function FilterChip({ active, onClick, children, count }) {
  return (
    <button className={`chip ${active ? 'active' : ''}`} onClick={onClick}>
      {children}
      {count != null && <span style={{ opacity: .65 }}>{count}</span>}
    </button>
  );
}

function HostsList({ state, onOpenHost, onAddHost, tenantId, density }) {
  const allHosts = window.HOSTS;
  const [selected, setSelected] = React.useState(new Set());
  const [filter, setFilter] = React.useState('all');
  const [query, setQuery] = React.useState('');

  const filteredHosts = React.useMemo(() => {
    let h = allHosts;
    if (filter === 'expiring') h = h.filter(x => x.cert === 'expiring' || x.cert === 'renewing' || x.cert === 'expired');
    if (filter === 'errors')   h = h.filter(x => x.codes.some(c => c[3] > 5));
    if (filter === 'cache')    h = h.filter(x => x.cache);
    if (filter === 'ratelimited') h = h.filter(x => x.rl);
    if (query) h = h.filter(x => x.host.includes(query.toLowerCase()) || x.up.includes(query.toLowerCase()));
    return h;
  }, [filter, query, allHosts]);

  const toggleAll = () => {
    if (selected.size === filteredHosts.length) setSelected(new Set());
    else setSelected(new Set(filteredHosts.map(h => h.host)));
  };
  const toggleOne = (host) => {
    const n = new Set(selected);
    if (n.has(host)) n.delete(host); else n.add(host);
    setSelected(n);
  };
  const allChecked = filteredHosts.length > 0 && selected.size === filteredHosts.length;
  const someChecked = selected.size > 0 && !allChecked;

  // Reset selection when tenant changes
  React.useEffect(() => { setSelected(new Set()); }, [tenantId]);

  return (
    <div className="page" style={{ position: 'relative' }}>
      <div className="page-hd">
        <div className="top">
          <h1>Hosts</h1>
          <span className="sub">· {window.HOST_COUNT_TOTAL.toLocaleString()} total</span>
          <div className="actions">
            <button className="btn sm" title="Refresh"><I.refresh /></button>
            <button className="btn sm"><I.download />Export</button>
            <button className="btn primary sm" onClick={onAddHost}><I.plus />Add host <Kbd>A</Kbd></button>
          </div>
        </div>
        <div className="toolbar">
          <div style={{ position: 'relative', minWidth: 260 }}>
            <I.search size={13} style={{ position: 'absolute', left: 9, top: 9, color: 'var(--muted)' }} />
            <input
              className="input mono"
              placeholder="Filter by hostname or upstream…"
              style={{ paddingLeft: 28, height: 28 }}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <FilterChip active={filter === 'all'} onClick={() => setFilter('all')}>All <span style={{ opacity: .65 }}>{allHosts.length}</span></FilterChip>
          <FilterChip active={filter === 'expiring'} onClick={() => setFilter('expiring')}>Cert ≤ 14d <span style={{ opacity: .65 }}>{allHosts.filter(x => ['expiring','renewing','expired'].includes(x.cert)).length}</span></FilterChip>
          <FilterChip active={filter === 'errors'} onClick={() => setFilter('errors')}>5xx errors <span style={{ opacity: .65 }}>{allHosts.filter(x => x.codes.some(c => c[3] > 5)).length}</span></FilterChip>
          <FilterChip active={filter === 'cache'} onClick={() => setFilter('cache')}>Cached <span style={{ opacity: .65 }}>{allHosts.filter(x => x.cache).length}</span></FilterChip>
          <FilterChip active={filter === 'ratelimited'} onClick={() => setFilter('ratelimited')}>Rate-limited <span style={{ opacity: .65 }}>{allHosts.filter(x => x.rl).length}</span></FilterChip>
          <span style={{ flex: 1 }} />
          <button className="btn sm ghost"><I.filter />More filters</button>
        </div>
      </div>

      {state === 'loading' && <HostsLoading />}
      {state === 'empty'   && <HostsEmpty onAddHost={onAddHost} />}
      {state === 'error'   && <HostsError />}
      {state === 'ok' && (
        <>
          <div className="tbl-wrap scroll">
            <table className="tbl">
              <thead>
                <tr>
                  <th className="check">
                    <input
                      type="checkbox"
                      className={`cb ${someChecked ? 'indet' : ''}`}
                      checked={allChecked}
                      onChange={toggleAll}
                      aria-label="Select all"
                    />
                  </th>
                  <th>Hostname</th>
                  <th>Upstream</th>
                  <th>Cert</th>
                  <th>Status · 5 min</th>
                  <th className="num">RPS</th>
                  <th>Cache</th>
                  <th>Rate</th>
                  <th>ACL</th>
                  <th className="acts" />
                </tr>
              </thead>
              <tbody>
                {filteredHosts.map((h) => (
                  <tr key={h.host} className={selected.has(h.host) ? 'selected' : ''}>
                    <td className="check">
                      <input
                        type="checkbox"
                        className="cb"
                        checked={selected.has(h.host)}
                        onChange={() => toggleOne(h.host)}
                        aria-label={`Select ${h.host}`}
                      />
                    </td>
                    <td className="host">
                      <button onClick={() => onOpenHost(h)} style={{ display: 'inline-flex', alignItems: 'center', color: 'inherit', font: 'inherit', textAlign: 'left' }}>
                        <I.lock size={11} className={`lock ${h.cert !== 'expired' && h.cert !== 'pending' ? 'active' : ''}`} />
                        {h.host}
                      </button>
                    </td>
                    <td className="up">→ {h.up}</td>
                    <td><CertPill status={h.cert} days={h.certDays} /></td>
                    <td><Sparkline codes={h.codes} /></td>
                    <td className="num mono">{h.rps ? h.rps.toLocaleString() : '—'}</td>
                    <td>{h.cache ? <span className="chip">{h.cache}</span> : <span className="muted">—</span>}</td>
                    <td>{h.rl ? <span className="chip">{h.rl}</span> : <span className="muted">—</span>}</td>
                    <td>{h.acl === 'deny' ? <span className="chip" style={{ color: 'var(--danger)', borderColor: 'var(--danger-border)' }}>deny</span> : h.acl ? <span className="chip">allow</span> : <span className="muted">—</span>}</td>
                    <td className="acts">
                      <button className="btn icon ghost sm" aria-label="More"><I.dots /></button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div style={{ height: 28, display: 'flex', alignItems: 'center', padding: '0 20px', borderTop: '1px solid var(--border)', fontSize: 11.5, color: 'var(--muted)', flexShrink: 0 }}>
            Showing <span className="mono" style={{ color: 'var(--fg-2)', margin: '0 4px' }}>{filteredHosts.length}</span>
            of <span className="mono" style={{ color: 'var(--fg-2)', margin: '0 4px' }}>{window.HOST_COUNT_TOTAL.toLocaleString()}</span>
            <span style={{ flex: 1 }} />
            <span>Press <Kbd>↑</Kbd><Kbd>↓</Kbd> to navigate · <Kbd>↵</Kbd> to open · <Kbd>⌘</Kbd><Kbd>K</Kbd> for actions</span>
          </div>

          {selected.size > 0 && (
            <div className="bulk">
              <span className="count">{selected.size} selected</span>
              <span className="sep" />
              <button className="btn sm"><I.refresh />Reissue certs</button>
              <button className="btn sm"><I.copy />Clone</button>
              <button className="btn sm"><I.download />Export</button>
              <button className="btn sm danger"><I.trash />Delete…</button>
              <span className="sep" />
              <button className="btn icon ghost sm" onClick={() => setSelected(new Set())} aria-label="Clear">
                <I.x />
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function HostsLoading() {
  return (
    <div className="tbl-wrap scroll">
      <table className="tbl">
        <thead>
          <tr>
            <th className="check" />
            <th>Hostname</th>
            <th>Upstream</th>
            <th>Cert</th>
            <th>Status · 5 min</th>
            <th className="num">RPS</th>
            <th>Cache</th>
            <th>Rate</th>
            <th>ACL</th>
            <th className="acts" />
          </tr>
        </thead>
        <tbody>
          {Array.from({ length: 18 }).map((_, i) => (
            <tr key={i}>
              <td className="check"><div className="sk" style={{ width: 14, height: 14 }} /></td>
              <td><div className="sk" style={{ width: 180 + (i * 7) % 80, height: 12 }} /></td>
              <td><div className="sk" style={{ width: 160 + (i * 11) % 60, height: 12 }} /></td>
              <td><div className="sk" style={{ width: 76, height: 16, borderRadius: 4 }} /></td>
              <td><div className="sk" style={{ width: 48, height: 16 }} /></td>
              <td className="num"><div className="sk" style={{ width: 36, height: 12, marginLeft: 'auto' }} /></td>
              <td><div className="sk" style={{ width: 54, height: 16, borderRadius: 4 }} /></td>
              <td><div className="sk" style={{ width: 36, height: 16, borderRadius: 4 }} /></td>
              <td><div className="sk" style={{ width: 36, height: 16, borderRadius: 4 }} /></td>
              <td className="acts"><div className="sk" style={{ width: 18, height: 16, marginLeft: 'auto' }} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function HostsEmpty({ onAddHost }) {
  return (
    <div className="empty">
      <div className="glyph"><I.globe size={20} style={{ color: 'var(--muted)' }} /></div>
      <h3>No hosts yet</h3>
      <p>A host maps an external hostname to an internal upstream. Kalista provisions TLS automatically via Let's Encrypt the first time the host receives a request.</p>
      <div className="flex" style={{ gap: 6 }}>
        <button className="btn primary" onClick={onAddHost}><I.plus />Add your first host <Kbd>A</Kbd></button>
        <button className="btn"><I.book />Read the docs</button>
      </div>
    </div>
  );
}

function HostsError() {
  return (
    <div className="empty">
      <div className="glyph" style={{ background: 'var(--danger-bg)', borderColor: 'var(--danger-border)' }}>
        <I.alert size={20} style={{ color: 'var(--danger)' }} />
      </div>
      <h3>Couldn't load hosts</h3>
      <p>The control plane API returned <span className="mono">503 Service Unavailable</span>. The data plane is still serving traffic — this only affects the admin UI.</p>
      <div className="flex" style={{ gap: 6 }}>
        <button className="btn primary"><I.refresh />Retry</button>
        <button className="btn"><I.pulse />Open status page</button>
      </div>
      <div className="banner danger" style={{ marginTop: 12, maxWidth: 420 }}>
        <I.info />
        <span className="grow">Last successful sync <span className="mono">2 min ago</span>. Showing cached snapshot may differ from current state.</span>
      </div>
    </div>
  );
}

window.HostsList = HostsList;
