// Add / Edit Host wizard. Renders as either a centered dialog or a full-page
// sheet — picked via tweak. The Review step is a Terraform-plan-style diff,
// unified or split per tweak. Shown for both create and edit; for create the
// "before" side is empty.

const WIZ_STEPS = [
  { id: 'domain',  label: 'Domain' },
  { id: 'upstream', label: 'Upstream' },
  { id: 'tls',     label: 'TLS' },
  { id: 'headers', label: 'Headers' },
  { id: 'cache',   label: 'Cache' },
  { id: 'rate',    label: 'Rate limit' },
  { id: 'acl',     label: 'ACL' },
  { id: 'review',  label: 'Review' },
];

// Sample diff: editing api.prod.acme.com. Lines marked +/-/~ with line numbers.
const DIFF_HUNKS = [
  { hdr: 'host "api.prod.acme.com"', lines: [
    { mk: ' ', tx: 'host "api.prod.acme.com" {' },
    { mk: ' ', tx: '  upstream {' },
    { mk: '-', tx: '    target = "api-gateway-v1.prod.svc:8443"' },
    { mk: '+', tx: '    target = "api-gateway.prod.svc:8443"' },
    { mk: '~', tx: '    health_check.interval = "5s"  # was "10s"' },
    { mk: ' ', tx: '    health_check.path = "/healthz"' },
    { mk: ' ', tx: '  }' },
    { mk: ' ', tx: '  tls {' },
    { mk: ' ', tx: '    provider = "letsencrypt"' },
    { mk: '~', tx: '    challenge = "http-01"  # was "tls-alpn-01"' },
    { mk: ' ', tx: '  }' },
  ]},
  { hdr: 'rate_limit "api.prod.acme.com"', lines: [
    { mk: '+', tx: 'rate_limit "api.prod.acme.com" {' },
    { mk: '+', tx: '  per     = "second"' },
    { mk: '+', tx: '  burst   = 2000' },
    { mk: '+', tx: '  key     = "remote_addr"' },
    { mk: '+', tx: '  action  = "drop"' },
    { mk: '+', tx: '}' },
  ]},
  { hdr: 'headers "api.prod.acme.com"', lines: [
    { mk: ' ', tx: 'headers "api.prod.acme.com" {' },
    { mk: ' ', tx: '  request {' },
    { mk: '+', tx: '    set "X-Request-ID" = uuid_v4()' },
    { mk: ' ', tx: '    set "X-Forwarded-For" = client_ip' },
    { mk: ' ', tx: '  }' },
    { mk: '-', tx: '  response.set "Server" = "kalista"' },
    { mk: ' ', tx: '}' },
  ]},
];

function DiffStats() {
  let add = 0, rem = 0, mod = 0;
  DIFF_HUNKS.forEach(h => h.lines.forEach(l => {
    if (l.mk === '+') add++;
    else if (l.mk === '-') rem++;
    else if (l.mk === '~') mod++;
  }));
  return { add, rem, mod };
}

function Diff({ split }) {
  const { add, rem, mod } = DiffStats();
  return (
    <div className="diff">
      <div className="diff-hd">
        <span style={{ fontWeight: 500, color: 'var(--fg)' }}>plan</span>
        <span className="muted">3 resources changed</span>
        <span style={{ flex: 1 }} />
        <span className="stat add"><strong>+{add}</strong> added</span>
        <span className="stat mod"><strong>~{mod}</strong> changed</span>
        <span className="stat rem"><strong>−{rem}</strong> removed</span>
      </div>

      <div className="diff-body">
        {DIFF_HUNKS.map((h, hi) => (
          <React.Fragment key={hi}>
            <div className="diff-line hdr">
              <span className="ln" />
              <span className="mk">@@</span>
              <span className="tx">{h.hdr}</span>
            </div>
            {!split && h.lines.map((l, i) => {
              const cls = l.mk === '+' ? 'add' : l.mk === '-' ? 'rem' : l.mk === '~' ? 'mod' : '';
              return (
                <div key={i} className={`diff-line ${cls}`}>
                  <span className="ln">{i + 1}</span>
                  <span className="mk">{l.mk === ' ' ? '' : l.mk}</span>
                  <span className="tx">{l.tx}</span>
                </div>
              );
            })}
            {split && (
              <div className="diff-split">
                <div>
                  {h.lines.filter(l => l.mk !== '+').map((l, i) => {
                    const cls = l.mk === '-' ? 'rem' : l.mk === '~' ? 'mod' : '';
                    return (
                      <div key={i} className={`diff-line ${cls}`}>
                        <span className="ln">{i + 1}</span>
                        <span className="mk">{l.mk === ' ' ? '' : l.mk === '~' ? '~' : '−'}</span>
                        <span className="tx">{l.tx.replace(/  # was.*/, '')}</span>
                      </div>
                    );
                  })}
                </div>
                <div>
                  {h.lines.filter(l => l.mk !== '-').map((l, i) => {
                    const cls = l.mk === '+' ? 'add' : l.mk === '~' ? 'mod' : '';
                    return (
                      <div key={i} className={`diff-line ${cls}`}>
                        <span className="ln">{i + 1}</span>
                        <span className="mk">{l.mk === ' ' ? '' : l.mk === '~' ? '~' : '+'}</span>
                        <span className="tx">{l.tx.replace(/  # was.*/, '')}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </React.Fragment>
        ))}
      </div>
    </div>
  );
}

function WizField({ label, hint, children }) {
  return (
    <div className="field">
      <label className="field-label">{label}</label>
      {children}
      {hint && <span className="field-hint">{hint}</span>}
    </div>
  );
}

function WizBody({ step, hostDraft, setHostDraft, diffSplit }) {
  if (step === 'domain') return (
    <div className="wiz-form">
      <h2>Domain</h2>
      <div className="lead">The hostname Kalista serves on. Kalista will request a TLS cert for it on first request.</div>
      <WizField label="Hostname" hint="One hostname per host. Wildcards (*.acme.com) require DNS-01.">
        <input className="input mono lg" value={hostDraft.host} onChange={(e) => setHostDraft({ ...hostDraft, host: e.target.value })} placeholder="api.prod.acme.com" />
      </WizField>
      <div className="row">
        <WizField label="Listen port"><input className="input mono" defaultValue="443" /></WizField>
        <WizField label="Protocol">
          <div className="flex" style={{ gap: 6 }}>
            <button className="chip active">HTTPS</button>
            <button className="chip">HTTP→HTTPS</button>
            <button className="chip">HTTP only</button>
          </div>
        </WizField>
      </div>
      <WizField label="Aliases" hint="Comma-separated. Each alias gets its own cert (or one multi-SAN cert).">
        <input className="input mono" defaultValue="www.api.prod.acme.com" />
      </WizField>
    </div>
  );

  if (step === 'upstream') return (
    <div className="wiz-form">
      <h2>Upstream</h2>
      <div className="lead">Where Kalista forwards traffic for this host. Kubernetes service DNS, raw IP:port, or a named upstream pool.</div>
      <WizField label="Target" hint="DNS, IP:port, or unix:/path. Use named upstream for load-balanced pools.">
        <div className="flex" style={{ gap: 6 }}>
          <input className="input mono lg" value={hostDraft.up} onChange={(e) => setHostDraft({ ...hostDraft, up: e.target.value })} placeholder="api-gateway.prod.svc:8443" />
          <button className="btn"><I.layers />Use pool…</button>
        </div>
      </WizField>
      <div className="row">
        <WizField label="Connect timeout"><input className="input mono" defaultValue="2s" /></WizField>
        <WizField label="Read timeout"><input className="input mono" defaultValue="30s" /></WizField>
      </div>
      <WizField label="Health check" hint="Active health-check. Tcp-only is cheaper but less informative than http.">
        <div className="flex" style={{ gap: 6 }}>
          <button className="chip">off</button>
          <button className="chip">tcp</button>
          <button className="chip active">http · /healthz</button>
          <button className="chip">grpc</button>
        </div>
      </WizField>
      <div className="row">
        <WizField label="Interval"><input className="input mono" defaultValue="5s" /></WizField>
        <WizField label="Expected status"><input className="input mono" defaultValue="2xx" /></WizField>
      </div>
    </div>
  );

  if (step === 'tls') return (
    <div className="wiz-form">
      <h2>TLS</h2>
      <div className="lead">Auto-provisioned via Let's Encrypt by default. Override per-host for specific provider, custom cert, or DNS-01.</div>
      <WizField label="Provider">
        <div className="flex" style={{ gap: 6 }}>
          <button className="chip active">Let's Encrypt</button>
          <button className="chip">ZeroSSL</button>
          <button className="chip">Custom upload</button>
        </div>
      </WizField>
      <WizField label="Challenge">
        <div className="flex" style={{ gap: 6 }}>
          <button className="chip active">HTTP-01</button>
          <button className="chip">TLS-ALPN-01</button>
          <button className="chip">DNS-01</button>
        </div>
      </WizField>
      <div className="row">
        <WizField label="Minimum TLS version">
          <select className="input"><option>TLS 1.2</option><option selected>TLS 1.3</option></select>
        </WizField>
        <WizField label="Auto-renew threshold"><input className="input mono" defaultValue="30 days before expiry" /></WizField>
      </div>
      <WizField label="HSTS">
        <div className="flex" style={{ gap: 6 }}>
          <button className="chip">off</button>
          <button className="chip active">31536000; includeSubDomains</button>
          <button className="chip">+ preload</button>
        </div>
      </WizField>
    </div>
  );

  if (step === 'headers') return (
    <div className="wiz-form">
      <h2>Headers</h2>
      <div className="lead">Add, override, or strip headers in both directions.</div>
      <div className="section-label" style={{ marginBottom: 8 }}>Request — client → upstream</div>
      <div style={{ marginBottom: 16, border: '1px solid var(--border)', borderRadius: 7, overflow: 'hidden' }}>
        {[
          ['set', 'X-Forwarded-For', '${client_ip}'],
          ['set', 'X-Forwarded-Proto', '${scheme}'],
          ['set', 'X-Request-ID', '${uuid_v4()}'],
        ].map((row, i) => (
          <div key={i} style={{ display: 'grid', gridTemplateColumns: '70px 1fr 1fr 28px', alignItems: 'center', gap: 8, padding: '6px 10px', borderBottom: i < 2 ? '1px solid var(--border)' : 0 }}>
            <Pill>{row[0]}</Pill>
            <input className="input mono" defaultValue={row[1]} style={{ height: 26 }} />
            <input className="input mono" defaultValue={row[2]} style={{ height: 26 }} />
            <button className="btn icon ghost sm"><I.x /></button>
          </div>
        ))}
      </div>
      <button className="btn sm"><I.plus />Add header rule</button>
    </div>
  );

  if (step === 'cache') return (
    <div className="wiz-form">
      <h2>Cache</h2>
      <div className="lead">Edge caching with stale-while-revalidate. Off by default for API hosts.</div>
      <WizField label="Mode">
        <div className="flex" style={{ gap: 6 }}>
          <button className="chip active">off</button>
          <button className="chip">honor Cache-Control</button>
          <button className="chip">override</button>
        </div>
      </WizField>
      <div className="muted" style={{ fontSize: 12, padding: 12, background: 'var(--surface-2)', borderRadius: 7 }}>
        Caching is off. Skipping to the next step is safe — you can enable it later from the host detail page.
      </div>
    </div>
  );

  if (step === 'rate') return (
    <div className="wiz-form">
      <h2>Rate limit</h2>
      <div className="lead">Token-bucket per key, applied at the edge.</div>
      <div className="row">
        <WizField label="Burst"><input className="input mono" defaultValue="2000" /></WizField>
        <WizField label="Per">
          <div className="flex" style={{ gap: 6 }}>
            <button className="chip">minute</button>
            <button className="chip active">second</button>
            <button className="chip">10s</button>
          </div>
        </WizField>
      </div>
      <WizField label="Key" hint="What to bucket by. remote_addr is the most common.">
        <div className="flex" style={{ gap: 6 }}>
          <button className="chip active">remote_addr</button>
          <button className="chip">header X-API-Key</button>
          <button className="chip">cookie session</button>
          <button className="chip">custom…</button>
        </div>
      </WizField>
      <WizField label="When exceeded">
        <div className="flex" style={{ gap: 6 }}>
          <button className="chip">return 429</button>
          <button className="chip active">drop connection</button>
          <button className="chip">log only</button>
        </div>
      </WizField>
    </div>
  );

  if (step === 'acl') return (
    <div className="wiz-form">
      <h2>ACL</h2>
      <div className="lead">Allow/deny IP ranges at the edge before the upstream sees the request.</div>
      <WizField label="Default policy">
        <div className="flex" style={{ gap: 6 }}>
          <button className="chip active">allow</button>
          <button className="chip">deny</button>
        </div>
      </WizField>
      <WizField label="Allow list" hint="CIDR ranges, one per line.">
        <textarea className="input mono" rows="3" style={{ height: 'auto', padding: 8 }} defaultValue={'10.0.0.0/8\n192.168.0.0/16\n2a01:4f8::/29'} />
      </WizField>
      <WizField label="Deny list">
        <textarea className="input mono" rows="2" style={{ height: 'auto', padding: 8 }} defaultValue={'45.142.213.0/24'} />
      </WizField>
    </div>
  );

  if (step === 'review') return (
    <div className="wiz-form">
      <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', marginBottom: 14 }}>
        <div>
          <h2>Review</h2>
          <div className="lead" style={{ marginBottom: 0 }}>Plan generated from your changes. Nothing is applied until you stage and apply.</div>
        </div>
        <div className="flex" style={{ gap: 6 }}>
          <button className={`chip ${!diffSplit ? 'active' : ''}`}>Unified</button>
          <button className={`chip ${diffSplit ? 'active' : ''}`}>Split</button>
        </div>
      </div>
      <div className="banner info" style={{ marginBottom: 14 }}>
        <I.info />
        <span className="grow">
          This will <span className="mono" style={{ fontWeight: 600 }}>stage</span> a draft. The data plane keeps serving the current config until an operator applies the batch.
          {' '}<a href="#" style={{ color: 'inherit', textDecoration: 'underline', textUnderlineOffset: 2 }}>What's a draft?</a>
        </span>
      </div>
      <Diff split={diffSplit} />
    </div>
  );

  return null;
}

function WizardContent({ mode, hostDraft, setHostDraft, stepIdx, setStepIdx, onClose, onApply, diffSplit }) {
  const step = WIZ_STEPS[stepIdx];
  return (
    <>
      <header style={{ height: 52, padding: '0 20px', borderBottom: '1px solid var(--border)', display: 'flex', alignItems: 'center', gap: 12, flexShrink: 0 }}>
        <I.globe size={14} style={{ color: 'var(--muted)' }} />
        <span style={{ fontSize: 14, fontWeight: 600 }}>{mode === 'edit' ? 'Edit host' : 'Add host'}</span>
        {hostDraft.host && <span className="mono muted" style={{ fontSize: 12.5 }}>{hostDraft.host}</span>}
        <span style={{ flex: 1 }} />
        <span className="muted" style={{ fontSize: 11.5 }}>Step {stepIdx + 1} of {WIZ_STEPS.length} · {step.label}</span>
        <button className="btn icon ghost sm" onClick={onClose} aria-label="Close"><I.x /></button>
      </header>
      <div className="wiz">
        <nav className="wiz-nav">
          {WIZ_STEPS.map((s, i) => (
            <button key={s.id}
                    className={`wiz-step ${i === stepIdx ? 'active' : i < stepIdx ? 'done' : ''}`}
                    onClick={() => setStepIdx(i)}>
              <span className="n">{i < stepIdx ? <I.check size={11} /> : i + 1}</span>
              {s.label}
            </button>
          ))}
        </nav>
        <div className="wiz-body">
          <WizBody step={step.id} hostDraft={hostDraft} setHostDraft={setHostDraft} diffSplit={diffSplit} />
          <div className="wiz-foot">
            <button className="btn ghost" onClick={onClose}>Cancel</button>
            <span style={{ flex: 1 }} />
            <span className="muted" style={{ fontSize: 11.5, marginRight: 8 }}><Kbd>⌘</Kbd><Kbd>↵</Kbd> to {stepIdx === WIZ_STEPS.length - 1 ? 'stage' : 'continue'}</span>
            <button className="btn" disabled={stepIdx === 0} onClick={() => setStepIdx(stepIdx - 1)}><I.chevL />Back</button>
            {stepIdx < WIZ_STEPS.length - 1
              ? <button className="btn primary" onClick={() => setStepIdx(stepIdx + 1)}>Continue<I.chevR /></button>
              : <button className="btn primary" onClick={onApply}>Stage draft <Kbd>⌘↵</Kbd></button>}
          </div>
        </div>
      </div>
    </>
  );
}

function AddHostWizard({ format, mode, onClose, diffSplit }) {
  const [stepIdx, setStepIdx] = React.useState(mode === 'edit' ? WIZ_STEPS.length - 1 : 0);
  const [hostDraft, setHostDraft] = React.useState(
    mode === 'edit'
      ? { host: 'api.prod.acme.com', up: 'api-gateway.prod.svc:8443' }
      : { host: '', up: '' },
  );

  if (format === 'dialog') {
    return (
      <>
        <div className="scrim" onClick={onClose} />
        <div className="dialog" role="dialog" aria-modal="true" style={{ width: 880, height: 580 }}>
          <WizardContent
            mode={mode}
            hostDraft={hostDraft} setHostDraft={setHostDraft}
            stepIdx={stepIdx} setStepIdx={setStepIdx}
            onClose={onClose} onApply={onClose}
            diffSplit={diffSplit}
          />
        </div>
      </>
    );
  }
  return (
    <>
      <div className="scrim" onClick={onClose} />
      <div className="sheet" role="dialog" aria-modal="true">
        <WizardContent
          mode={mode}
          hostDraft={hostDraft} setHostDraft={setHostDraft}
          stepIdx={stepIdx} setStepIdx={setStepIdx}
          onClose={onClose} onApply={onClose}
          diffSplit={diffSplit}
        />
      </div>
    </>
  );
}

window.AddHostWizard = AddHostWizard;
window.Diff = Diff;
