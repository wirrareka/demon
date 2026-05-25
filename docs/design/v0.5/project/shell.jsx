// App shell: sidebar + tenant switcher + topbar. The chrome that surrounds
// every surface. Keyboard-first; sidebar collapses to icon-only at 56px.

function TenantSwitcher({ tenant, onPick }) {
  const [open, setOpen] = React.useState(false);
  React.useEffect(() => {
    if (!open) return;
    const close = (e) => { if (!e.target.closest('.tenant-menu') && !e.target.closest('.tenant')) setOpen(false); };
    window.addEventListener('mousedown', close);
    return () => window.removeEventListener('mousedown', close);
  }, [open]);
  return (
    <div style={{ position: 'relative' }}>
      <button className="tenant" onClick={() => setOpen(v => !v)} aria-haspopup="menu" aria-expanded={open}>
        <span className="avatar">{tenant.initials}</span>
        <span className="meta">
          <span className="name">{tenant.name}</span>
          <span className="sub">v0.4.76 · 3 nodes</span>
        </span>
        <I.chev className="caret" />
      </button>
      {open && (
        <div className="tenant-menu" style={{
          position: 'absolute', top: 44, left: 8, right: 8,
          background: 'var(--surface)', border: '1px solid var(--border-2)',
          borderRadius: 8, boxShadow: 'var(--shadow-pop)', zIndex: 20, padding: 4,
        }}>
          <div style={{ fontSize: 10.5, color: 'var(--muted)', padding: '6px 8px', letterSpacing: '.06em', textTransform: 'uppercase', fontWeight: 600 }}>Tenants</div>
          {window.TENANTS.map(t => (
            <button key={t.id} className="side-item" onClick={() => { onPick(t); setOpen(false); }}>
              <span className="avatar" style={{ width: 18, height: 18, borderRadius: 4, fontSize: 9 }}>{t.initials}</span>
              <span className="lbl">{t.name}</span>
              {t.id === tenant.id && <I.check size={12} style={{ color: 'var(--muted)' }} />}
            </button>
          ))}
          <div style={{ borderTop: '1px solid var(--border)', marginTop: 4, paddingTop: 4 }}>
            <button className="side-item"><I.plus size={13} className="ic" /><span className="lbl">New tenant…</span></button>
            <button className="side-item"><I.settings size={13} className="ic" /><span className="lbl">Manage tenants</span></button>
          </div>
        </div>
      )}
    </div>
  );
}

function SidebarItem({ icon: Ic, label, active, badge, badgeKind, kbd, onClick }) {
  return (
    <button className={`side-item ${active ? 'active' : ''}`} onClick={onClick} title={label}>
      <Ic className="ic" />
      <span className="lbl">{label}</span>
      {badge != null && <span className={`badge ${badgeKind || ''}`}>{badge}</span>}
      {kbd && !badge && <Kbd>{kbd}</Kbd>}
    </button>
  );
}

function Sidebar({ surface, setSurface, draftCount, tenant, setTenant }) {
  return (
    <aside className="sidebar">
      <TenantSwitcher tenant={tenant} onPick={setTenant} />

      <div className="side-section" style={{ paddingTop: 8 }}>
        <div className="lbl">Routing</div>
        <SidebarItem icon={I.globe}  label="Hosts"       kbd="g h" active={surface === 'hosts' || surface === 'host-detail'} onClick={() => setSurface('hosts')} />
        <SidebarItem icon={I.server} label="Upstreams"   kbd="g u" active={surface === 'upstreams'} onClick={() => setSurface('upstreams')} />
        <SidebarItem icon={I.layers} label="TCP routes"            active={surface === 'tcp'} onClick={() => setSurface('tcp')} />
      </div>

      <div className="side-section">
        <div className="lbl">Security</div>
        <SidebarItem icon={I.cert}   label="Certificates" kbd="g c" active={surface === 'certs'} onClick={() => setSurface('certs')} />
        <SidebarItem icon={I.shield} label="ACLs"                  active={surface === 'acls'} onClick={() => setSurface('acls')} />
      </div>

      <div className="side-section">
        <div className="lbl">Observe</div>
        <SidebarItem icon={I.pulse}  label="Live logs"    kbd="g l" active={surface === 'logs'} onClick={() => setSurface('logs')} />
        <SidebarItem icon={I.history} label="Audit"                active={surface === 'audit'} onClick={() => setSurface('audit')} />
        <SidebarItem icon={I.bolt}   label="Metrics"               active={surface === 'metrics'} onClick={() => setSurface('metrics')} />
      </div>

      <div className="side-section">
        <div className="lbl">Pipeline</div>
        <SidebarItem icon={I.drafts} label="Drafts" badge={draftCount} badgeKind={draftCount > 0 ? 'warn' : ''} active={surface === 'drafts'} onClick={() => setSurface('drafts')} />
        <SidebarItem icon={I.bell}   label="Webhooks" />
      </div>

      <div style={{ flex: 1 }} />

      <div className="side-section" style={{ paddingTop: 0 }}>
        <SidebarItem icon={I.book}     label="Docs" />
        <SidebarItem icon={I.settings} label="Settings"   active={surface === 'settings'} onClick={() => setSurface('settings')} />
      </div>

      <div style={{ borderTop: '1px solid var(--border)', padding: 8 }}>
        <button className="side-item" title="tomas@acme.com">
          <span className="avatar" style={{ width: 22, height: 22, borderRadius: 5, fontSize: 10, background: 'var(--surface-3)', color: 'var(--fg)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center', border: '1px solid var(--border)' }}>TK</span>
          <span className="lbl">tomas@acme.com</span>
          <I.dots size={13} style={{ color: 'var(--muted)' }} />
        </button>
      </div>
    </aside>
  );
}

function Crumbs({ items }) {
  return (
    <nav className="crumbs" aria-label="breadcrumbs">
      {items.map((it, i) => {
        const isLast = i === items.length - 1;
        return (
          <React.Fragment key={i}>
            {i > 0 && <span className="sep">/</span>}
            <span className={isLast ? 'leaf' : 'home'}>
              {it.icon ? <it.icon size={13} /> : null}
              {it.label}
            </span>
          </React.Fragment>
        );
      })}
    </nav>
  );
}

function Topbar({ crumbs, onCmdK, onCollapse, theme, setTheme }) {
  const isMac = typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.platform);
  return (
    <header className="topbar">
      <button className="btn icon ghost sm" onClick={onCollapse} aria-label="Toggle sidebar">
        <I.side />
      </button>
      <span className="sep-v" />
      <Crumbs items={crumbs} />
      <button className="cmdk-trigger" onClick={onCmdK} aria-label="Open command palette">
        <I.search />
        <span>Search or jump to…</span>
        <span className="kbd-pair"><Kbd>{isMac ? '⌘' : 'Ctrl'}</Kbd><Kbd>K</Kbd></span>
      </button>
      <button className="btn icon ghost sm" aria-label="Notifications">
        <I.bell />
      </button>
      <button className="btn icon ghost sm" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')} aria-label="Toggle theme">
        {theme === 'dark' ? <I.sun /> : <I.moon />}
      </button>
    </header>
  );
}

window.Sidebar = Sidebar;
window.Topbar = Topbar;
window.Crumbs = Crumbs;
