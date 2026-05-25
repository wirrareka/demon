// Top-level app: state for surface, theme, density, sidebar, tweaks. Wires
// everything together. Loads the EDITMODE block so Tweaks persists.

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "theme": "dark",
  "accent": "mono",
  "density": "compact",
  "side": "expanded",
  "diffSplit": false,
  "surface": "hosts",
  "state": "ok"
}/*EDITMODE-END*/;

function App() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);
  const [tenant, setTenant] = React.useState(window.TENANTS[0]);
  const [openHost, setOpenHost] = React.useState(null);
  const [wizard, setWizard] = React.useState(null); // null | { format, mode }
  const [cmdkOpen, setCmdkOpen] = React.useState(false);

  // Apply tokens at the root element
  React.useEffect(() => {
    const root = document.documentElement;
    root.setAttribute('data-theme', t.theme);
    root.setAttribute('data-accent', t.accent);
    root.setAttribute('data-density', t.density);
  }, [t.theme, t.accent, t.density]);

  // Global shortcut: Cmd/Ctrl-K → cmdk; 'a' → add host; g h/u/c → nav
  const gPrefix = React.useRef(0);
  React.useEffect(() => {
    const onKey = (e) => {
      const isMod = e.metaKey || e.ctrlKey;
      if (isMod && e.key.toLowerCase() === 'k') { e.preventDefault(); setCmdkOpen(true); return; }
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
      if (e.key === 'a' && !wizard && !cmdkOpen) { e.preventDefault(); setWizard({ format: 'sheet', mode: 'create' }); return; }
      if (e.key === 'g') { gPrefix.current = Date.now(); return; }
      if (Date.now() - gPrefix.current < 1200) {
        if (e.key === 'h') { setTweak('surface', 'hosts'); setOpenHost(null); }
        if (e.key === 'u') setTweak('surface', 'upstreams');
        if (e.key === 'c') setTweak('surface', 'certs');
        if (e.key === 'l') setTweak('surface', 'logs');
        gPrefix.current = 0;
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [wizard, cmdkOpen, setTweak]);

  const surface = t.surface;
  const setSurface = (s) => { setTweak('surface', s); setOpenHost(null); };

  let crumbs = [{ icon: I.globe, label: tenant.name }, { label: 'Hosts' }];
  if (surface === 'upstreams') crumbs = [{ icon: I.globe, label: tenant.name }, { label: 'Upstreams' }];
  if (surface === 'certs')     crumbs = [{ icon: I.globe, label: tenant.name }, { label: 'Certificates' }];
  if (surface === 'logs')      crumbs = [{ icon: I.globe, label: tenant.name }, { label: 'Live logs' }];
  if (surface === 'drafts')    crumbs = [{ icon: I.globe, label: tenant.name }, { label: 'Drafts' }];
  if (openHost) crumbs = [{ icon: I.globe, label: tenant.name }, { label: 'Hosts' }, { label: openHost.host }];

  // Wizard mode label hints into the breadcrumb when open
  return (
    <div className="app" data-side={t.side}>
      <Sidebar
        surface={openHost ? 'hosts' : surface}
        setSurface={setSurface}
        draftCount={window.DRAFTS.length}
        tenant={tenant}
        setTenant={setTenant}
      />
      <div className="main">
        <Topbar
          crumbs={crumbs}
          onCmdK={() => setCmdkOpen(true)}
          onCollapse={() => setTweak('side', t.side === 'expanded' ? 'collapsed' : 'expanded')}
          theme={t.theme}
          setTheme={(v) => setTweak('theme', v)}
        />
        <main className="content">
          {openHost
            ? <HostDetail host={openHost} onBack={() => setOpenHost(null)} onEdit={() => setWizard({ format: 'sheet', mode: 'edit' })} />
            : (surface === 'hosts'
                ? <HostsList state={t.state}
                             onOpenHost={(h) => setOpenHost(h)}
                             onAddHost={() => setWizard({ format: 'sheet', mode: 'create' })}
                             tenantId={tenant.id}
                             density={t.density} />
                : <PlaceholderSurface surface={surface} />)
          }
        </main>
      </div>

      {wizard && (
        <AddHostWizard
          format={wizard.format}
          mode={wizard.mode}
          onClose={() => setWizard(null)}
          diffSplit={t.diffSplit}
        />
      )}

      <CmdK
        open={cmdkOpen}
        onClose={() => setCmdkOpen(false)}
        onJumpTo={(target) => { if (target.kind === 'host') setOpenHost(target.raw); }}
        onAddHost={() => setWizard({ format: 'sheet', mode: 'create' })}
      />

      <TweaksPanel title="Kalista · Tweaks">
        <TweakSection label="Theme & density">
          <TweakRadio  label="Theme"   value={t.theme}   options={['dark', 'light']} onChange={(v) => setTweak('theme', v)} />
          <TweakRadio  label="Density" value={t.density} options={['compact', 'extra']} onChange={(v) => setTweak('density', v)} />
          <TweakRadio  label="Sidebar" value={t.side}    options={['expanded', 'collapsed']} onChange={(v) => setTweak('side', v)} />
        </TweakSection>
        <TweakSection label="Accent">
          <TweakSelect label="Accent" value={t.accent}
                       options={[
                         { value: 'mono',    label: 'Mono — ink on paper' },
                         { value: 'cobalt',  label: 'Cobalt — deep blue' },
                         { value: 'solder',  label: 'Solder — copper amber' },
                         { value: 'pingora', label: 'Pingora — moss teal' },
                         { value: 'iris',    label: 'Iris — violet' },
                       ]}
                       onChange={(v) => setTweak('accent', v)} />
        </TweakSection>
        <TweakSection label="Surface">
          <TweakSelect label="Surface" value={t.surface}
                       options={[
                         { value: 'hosts',    label: 'Hosts list' },
                         { value: 'upstreams', label: 'Upstreams (stub)' },
                         { value: 'certs',     label: 'Certificates (stub)' },
                         { value: 'logs',      label: 'Live logs (stub)' },
                         { value: 'drafts',    label: 'Drafts (stub)' },
                       ]}
                       onChange={(v) => { setTweak('surface', v); setOpenHost(null); }} />
          <TweakRadio label="Hosts state" value={t.state}
                      options={['ok', 'loading', 'empty', 'error']}
                      onChange={(v) => setTweak('state', v)} />
        </TweakSection>
        <TweakSection label="Wizard">
          <TweakButton label="Open · sheet (create)"  onClick={() => setWizard({ format: 'sheet',  mode: 'create' })} />
          <TweakButton label="Open · dialog (edit)"   secondary onClick={() => setWizard({ format: 'dialog', mode: 'edit' })} />
          <TweakRadio  label="Diff view" value={t.diffSplit ? 'split' : 'unified'}
                       options={['unified', 'split']}
                       onChange={(v) => setTweak('diffSplit', v === 'split')} />
        </TweakSection>
        <TweakSection label="Other">
          <TweakButton label="Open Cmd-K" onClick={() => setCmdkOpen(true)} />
        </TweakSection>
      </TweaksPanel>
    </div>
  );
}

function PlaceholderSurface({ surface }) {
  const titles = {
    upstreams: 'Upstreams',
    certs: 'Certificates',
    logs: 'Live logs',
    drafts: 'Drafts',
    audit: 'Audit log',
    metrics: 'Metrics',
    tcp: 'TCP routes',
    acls: 'ACLs',
    settings: 'Settings',
  };
  return (
    <div className="page">
      <div className="page-hd">
        <div className="top">
          <h1>{titles[surface] || surface}</h1>
          <span className="sub">· this surface is stubbed in the prototype</span>
        </div>
      </div>
      <div className="empty">
        <div className="glyph"><I.layers size={20} style={{ color: 'var(--muted)' }} /></div>
        <h3>{titles[surface] || surface} surface</h3>
        <p>The redesign pass focused on Hosts, Host detail, the wizard, and Cmd-K. This surface inherits the same tokens, sidebar, topbar, and density rules. See <span className="mono">redesign-rationale.md</span> for the parity spec.</p>
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
