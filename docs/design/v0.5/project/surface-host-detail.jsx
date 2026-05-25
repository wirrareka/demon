// Host detail surface + Cert lifecycle Gantt.
// Gantt: horizontal lanes per cert version, segments by phase (issued/active/
// renewed/replaced/expires). Today line. Hover shows ACME order + retries.

function CertGantt() {
  const lc = window.CERT_LIFECYCLE;
  const start = lc.windowStart, end = lc.windowEnd;
  const span = end - start;
  const xOf = (day) => `${((day - start) / span) * 100}%`;
  const wOf = (from, to) => `${((Math.max(to, from + 0.4) - from) / span) * 100}%`;
  const todayPct = ((0 - start) / span) * 100;

  // Tick marks every 30 days within window
  const ticks = [];
  for (let d = Math.ceil(start / 30) * 30; d <= end; d += 30) ticks.push(d);

  const [hover, setHover] = React.useState(null);

  return (
    <div className="gantt">
      <div className="gantt-hd">
        <div style={{ paddingRight: 12, fontSize: 11, color: 'var(--muted)', fontWeight: 500 }}>
          <span className="section-label">Cert versions · {window.CERT_LIFECYCLE.host}</span>
        </div>
        <div className="scale" style={{ marginRight: 16 }}>
          {ticks.map(d => (
            <span key={d} className={`tick ${d === 0 ? 'today' : ''}`} style={{ left: xOf(d) }}>
              {d === 0 ? 'today' : (d > 0 ? `+${d}d` : `${d}d`)}
            </span>
          ))}
        </div>
      </div>
      {lc.versions.map((v) => (
        <div className="gantt-row" key={v.id}>
          <div className="meta">
            <div className="id">cert#{v.id} · {v.serial}</div>
            <div className="sub">{v.issuer} · SAN: {v.san[0]}{v.san.length > 1 ? ` +${v.san.length - 1}` : ''}</div>
          </div>
          <div className="lane" style={{ marginRight: 16 }}>
            <span className="today-line" style={{ left: `${todayPct}%` }} />
            {v.segments.map((s, i) => {
              if (s.kind === 'active') {
                return (
                  <div key={i} className="gantt-bar active"
                       style={{ left: xOf(Math.max(start, s.from)), width: wOf(Math.max(start, s.from), s.to) }}>
                    {s.to > 0 ? 'current cert' : 'previous cert'}
                  </div>
                );
              }
              if (s.kind === 'issued') {
                return (
                  <div key={i} className="gantt-mark issued"
                       style={{ left: `calc(${xOf(s.from)} - 4px)` }}
                       onMouseEnter={(e) => setHover({ x: e.clientX, y: e.clientY, label: `Issued · day ${s.from}` })}
                       onMouseLeave={() => setHover(null)}>
                    <span className="dot" />
                  </div>
                );
              }
              if (s.kind === 'renews') {
                return (
                  <div key={i} className="gantt-mark renews"
                       style={{ left: `calc(${xOf(s.from)} - 4px)` }}
                       onMouseEnter={(e) => setHover({ x: e.clientX, y: e.clientY, label: `Scheduled renewal · in ${s.from}d · 30d before expiry` })}
                       onMouseLeave={() => setHover(null)}>
                    <span className="dot" />
                  </div>
                );
              }
              if (s.kind === 'expires') {
                return (
                  <div key={i} className="gantt-mark expires"
                       style={{ left: `calc(${xOf(s.from)} - 4px)` }}
                       onMouseEnter={(e) => setHover({ x: e.clientX, y: e.clientY, label: `Expires · in ${s.from}d · 2026-07-04 14:32 UTC` })}
                       onMouseLeave={() => setHover(null)}>
                    <span className="dot" />
                  </div>
                );
              }
              if (s.kind === 'replaced') {
                return (
                  <div key={i} className="gantt-mark replaced"
                       style={{ left: `calc(${xOf(s.from)} - 4px)` }}
                       onMouseEnter={(e) => setHover({ x: e.clientX, y: e.clientY, label: `Replaced by next cert version` })}
                       onMouseLeave={() => setHover(null)}>
                    <span className="dot" />
                  </div>
                );
              }
              return null;
            })}
          </div>
        </div>
      ))}
      {/* Legend */}
      <div style={{ display: 'flex', gap: 16, padding: '10px 14px', borderTop: '1px solid var(--border)', fontSize: 11, color: 'var(--muted)' }}>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}><i style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--info)' }} />Issued</span>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}><i style={{ width: 14, height: 8, borderRadius: 2, background: 'color-mix(in oklch, var(--success) 12%, var(--bg))', border: '1px solid var(--success-border)' }} />Active</span>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}><i style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--warning)' }} />Renewal scheduled</span>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}><i style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--danger)' }} />Expires</span>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}><i style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--muted)' }} />Replaced</span>
        <span style={{ marginLeft: 'auto' }}>Hover any mark for ACME order URL + retries</span>
      </div>
      {hover && (
        <div className="tooltip" style={{ left: hover.x + 12, top: hover.y - 8, position: 'fixed' }}>
          {hover.label}
        </div>
      )}
    </div>
  );
}

function MiniStat({ label, value, sub, kind }) {
  return (
    <div style={{
      border: '1px solid var(--border)',
      borderRadius: 8,
      padding: '10px 14px',
      background: 'var(--surface)',
      minWidth: 0,
    }}>
      <div style={{ fontSize: 11, color: 'var(--muted)', letterSpacing: '.04em', textTransform: 'uppercase', fontWeight: 500 }}>{label}</div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 6, marginTop: 4 }}>
        <span className="mono tnum" style={{ fontSize: 22, fontWeight: 500, letterSpacing: '-0.01em', color: kind === 'danger' ? 'var(--danger)' : kind === 'warning' ? 'var(--warning)' : 'var(--fg)' }}>{value}</span>
        {sub && <span style={{ fontSize: 11.5, color: 'var(--muted)' }}>{sub}</span>}
      </div>
    </div>
  );
}

function HostDetail({ host, onBack, onEdit }) {
  const [tab, setTab] = React.useState('cert');
  return (
    <div className="page">
      <div className="page-hd">
        <div className="top">
          <button className="btn icon ghost sm" onClick={onBack} aria-label="Back"><I.chevL /></button>
          <h1 className="mono" style={{ fontSize: 15, fontWeight: 500, letterSpacing: '-0.005em' }}>{host.host}</h1>
          <Pill kind="success" dot>healthy</Pill>
          <CertPill status={host.cert} days={host.certDays} />
          <Pill>v0.4.76</Pill>
          <div className="actions">
            <button className="btn sm"><I.refresh />Force renew</button>
            <button className="btn sm"><I.copy />Clone</button>
            <button className="btn sm" onClick={onEdit}><I.edit />Edit</button>
            <button className="btn icon ghost sm"><I.dots /></button>
          </div>
        </div>
        <div className="toolbar muted" style={{ fontSize: 12.5 }}>
          <span>→ <span className="mono" style={{ color: 'var(--fg-2)' }}>{host.up}</span></span>
          <span className="sep-v" />
          <span>Edited <span className="mono" style={{ color: 'var(--fg-2)' }}>14m ago</span> by <span style={{ color: 'var(--fg-2)' }}>tomas</span></span>
          <span className="sep-v" />
          <span><span className="mono" style={{ color: 'var(--fg-2)' }}>{host.rps.toLocaleString()}</span> rps · last 5 min</span>
        </div>
      </div>

      <div className="tabs">
        <button className={`tab ${tab === 'overview' ? 'active' : ''}`} onClick={() => setTab('overview')}>Overview</button>
        <button className={`tab ${tab === 'cert' ? 'active' : ''}`}     onClick={() => setTab('cert')}>Cert lifecycle</button>
        <button className={`tab ${tab === 'logs' ? 'active' : ''}`}     onClick={() => setTab('logs')}>Live logs <span className="count tnum">stream</span></button>
        <button className={`tab ${tab === 'audit' ? 'active' : ''}`}    onClick={() => setTab('audit')}>Audit <span className="count tnum">37</span></button>
        <button className={`tab ${tab === 'metrics' ? 'active' : ''}`}  onClick={() => setTab('metrics')}>Metrics</button>
      </div>

      <div className="scroll" style={{ flex: 1, padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 16 }}>
        {tab === 'cert' && (
          <>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 10 }}>
              <MiniStat label="Current cert" value={`${host.certDays}d`} sub="until expiry" kind={host.certDays < 14 ? 'warning' : null} />
              <MiniStat label="Auto renewal" value="−30d" sub="before expiry" />
              <MiniStat label="ACME provider" value="Let's Encrypt" sub="HTTP-01" />
              <MiniStat label="Last failure" value="—" sub="9 renewals, 0 failures" />
            </div>
            <CertGantt />
            <div className="card">
              <div className="card-hd">
                <h3>ACME events</h3>
                <span className="muted" style={{ fontSize: 11.5 }}>most recent first</span>
              </div>
              <div>
                {[
                  { when: '2 days ago', type: 'issued', msg: 'cert#v3 issued · 03:8a:2f:c1:9d:e4 · valid 90d' },
                  { when: '2 days ago', type: 'auth',   msg: 'http-01 challenge solved · acme-v02.api.letsencrypt.org/order/12847' },
                  { when: '2 days ago', type: 'order',  msg: 'new-order · acme-v02.api.letsencrypt.org/order/12847 · 1 retry' },
                  { when: '62 days ago', type: 'rotated', msg: 'cert#v2 replaced by cert#v3' },
                  { when: '62 days ago', type: 'issued', msg: 'cert#v2 issued · 03:81:7e:44:2c:e1 · valid 90d' },
                ].map((e, i) => (
                  <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '8px 14px', borderBottom: i < 4 ? '1px solid var(--border)' : 0, fontSize: 12 }}>
                    <span className="mono muted" style={{ width: 96, flexShrink: 0, fontSize: 11.5 }}>{e.when}</span>
                    <Pill kind={e.type === 'issued' ? 'success' : e.type === 'order' ? 'info' : 'default'}>{e.type}</Pill>
                    <span className="mono" style={{ color: 'var(--fg-2)' }}>{e.msg}</span>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}

        {tab === 'overview' && (
          <>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 10 }}>
              <MiniStat label="Requests · 24h" value="412M" sub="+4.2%" />
              <MiniStat label="p99 latency" value="184ms" sub="−12ms" />
              <MiniStat label="5xx · 24h" value="0.07%" sub="288 of 412M" />
              <MiniStat label="Cache hit rate" value="—" sub="cache off" />
            </div>
            <div className="card">
              <div className="card-hd"><h3>Configuration</h3><button className="btn sm" onClick={onEdit}><I.edit />Edit</button></div>
              <div style={{ padding: '4px 0' }}>
                {[
                  ['Hostname', host.host],
                  ['Upstream', host.up],
                  ['Health check', 'tcp · 5s · /healthz expected 2xx'],
                  ['Request headers', 'X-Forwarded-For, X-Request-ID, Server-Timing'],
                  ['Rate limit', host.rl || '—'],
                  ['ACL', host.acl ? `${host.acl} · 10.0.0.0/8, 192.168.0.0/16` : '—'],
                  ['Cache', host.cache || '—'],
                ].map(([k, v]) => (
                  <div key={k} style={{ display: 'grid', gridTemplateColumns: '180px 1fr', alignItems: 'center', padding: '8px 14px', borderBottom: '1px solid var(--border)', fontSize: 12.5 }}>
                    <span className="muted">{k}</span>
                    <span className="mono" style={{ color: 'var(--fg)' }}>{v}</span>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}

        {tab === 'logs' && (
          <div className="card" style={{ padding: 0 }}>
            <div className="card-hd">
              <h3 style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                <span style={{ width: 6, height: 6, borderRadius: '50%', background: 'var(--success)', boxShadow: '0 0 0 3px color-mix(in oklch, var(--success), transparent 70%)' }} />
                Tailing live · {host.host}
              </h3>
              <div className="flex" style={{ gap: 6 }}>
                <button className="btn sm ghost">Pause</button>
                <button className="btn sm"><I.filter />Filters · 2</button>
              </div>
            </div>
            <div style={{ padding: 8, fontFamily: 'JetBrains Mono, monospace', fontSize: 11.5, lineHeight: 1.55, maxHeight: 360, overflow: 'auto' }} className="scroll">
              {[
                ['14:32:08.144', '200', '4.2ms',  'GET',    '/v1/users/4811',           '203.0.113.41', '512B'],
                ['14:32:08.139', '200', '12.7ms', 'POST',   '/v1/charges',              '198.51.100.7', '1.3KB'],
                ['14:32:08.121', '304', '0.8ms',  'GET',    '/static/app-3a7c.js',      '203.0.113.18', '0B'],
                ['14:32:08.097', '200', '8.4ms',  'GET',    '/v1/products?limit=50',    '203.0.113.41', '24KB'],
                ['14:32:08.072', '404', '2.1ms',  'GET',    '/.env',                    '45.142.213.99','142B'],
                ['14:32:08.041', '200', '6.0ms',  'GET',    '/v1/users/me',             '203.0.113.41', '288B'],
                ['14:32:07.998', '503', '102ms',  'POST',   '/v1/checkout/sessions',    '198.51.100.7', '64B'],
                ['14:32:07.961', '200', '14.2ms', 'POST',   '/v1/customers',            '198.51.100.7', '912B'],
                ['14:32:07.945', '200', '3.4ms',  'GET',    '/v1/users/me',             '203.0.113.18', '288B'],
              ].map((row, i) => {
                const status = row[1];
                const color = status.startsWith('2') ? 'var(--success)' : status.startsWith('3') ? 'var(--muted)' : status.startsWith('4') ? 'var(--warning)' : 'var(--danger)';
                return (
                  <div key={i} style={{ display: 'grid', gridTemplateColumns: '90px 36px 60px 50px 1fr 110px 60px', gap: 12, padding: '3px 8px', borderRadius: 4 }}>
                    <span style={{ color: 'var(--muted)' }}>{row[0]}</span>
                    <span style={{ color, fontWeight: 600 }}>{row[1]}</span>
                    <span style={{ color: 'var(--fg-2)' }} className="tnum">{row[2]}</span>
                    <span style={{ color: 'var(--fg-2)' }}>{row[3]}</span>
                    <span style={{ color: 'var(--fg)', overflow: 'hidden', textOverflow: 'ellipsis' }}>{row[4]}</span>
                    <span style={{ color: 'var(--muted)' }}>{row[5]}</span>
                    <span style={{ color: 'var(--muted)' }} className="tnum">{row[6]}</span>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {tab === 'audit' && (
          <div className="card">
            <div className="card-hd"><h3>Audit · {host.host}</h3><button className="btn sm"><I.download />Export</button></div>
            <div>
              {[
                ['14m ago',  'config', 'tomas',  'Updated rate limit · 1000/s → 2000/s'],
                ['2d ago',   'cert',   'system', 'Cert reissued · cert#v3 · LE R11'],
                ['8d ago',   'config', 'lukas',  'Added header X-Request-ID propagation'],
                ['41d ago',  'config', 'tomas',  'Switched upstream from api-gateway-v1 → api-gateway · health-check tcp'],
                ['62d ago',  'cert',   'system', 'Cert renewed · cert#v3 replaces cert#v2'],
                ['188d ago', 'create', 'tomas',  'Created host'],
              ].map((row, i) => (
                <div key={i} style={{ display: 'grid', gridTemplateColumns: '96px 84px 120px 1fr', gap: 12, padding: '8px 14px', borderBottom: i < 5 ? '1px solid var(--border)' : 0, fontSize: 12 }}>
                  <span className="mono muted" style={{ fontSize: 11.5 }}>{row[0]}</span>
                  <Pill>{row[1]}</Pill>
                  <span className="mono" style={{ color: 'var(--fg-2)' }}>{row[2]}</span>
                  <span style={{ color: 'var(--fg)' }}>{row[3]}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {tab === 'metrics' && (
          <div className="empty" style={{ minHeight: 280 }}>
            <div className="glyph"><I.bolt size={20} style={{ color: 'var(--muted)' }} /></div>
            <h3>Metrics</h3>
            <p>Recharts panels: RPS, p50/p95/p99 latency, status-class breakdown, upstream connect time. Linked to Prometheus retention window.</p>
          </div>
        )}
      </div>
    </div>
  );
}

window.HostDetail = HostDetail;
