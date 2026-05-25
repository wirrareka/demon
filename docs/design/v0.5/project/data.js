// Operator-language fixtures. No "Item One" placeholders.
// The Hosts table needs to look like 500-row prod, not a demo.

window.TENANTS = [
  { id: 'acme-prod', name: 'acme · prod', initials: 'AP', color: '#a3a3a3' },
  { id: 'acme-staging', name: 'acme · staging', initials: 'AS', color: '#737373' },
  { id: 'acme-dev', name: 'acme · dev', initials: 'AD', color: '#525252' },
  { id: 'internal', name: 'internal', initials: 'IN', color: '#404040' },
  { id: 'edge-eu', name: 'edge-eu', initials: 'EU', color: '#262626' },
];

// Cert status: active | renewing | expiring | expired | pending
// Status code mix encodes the sparkline (last 5 buckets, each value is a
// short array of [2xx, 3xx, 4xx, 5xx] fractions out of 100).
const codes = (a, b, c, d) => [a, b, c, d];

window.HOSTS = [
  { host: 'api.prod.acme.com',                up: 'api-gateway.prod.svc:8443',     cert: 'active',    certDays: 64, rps: 4823, codes: [codes(96,2,2,0),codes(95,2,3,0),codes(94,2,3,1),codes(95,2,3,0),codes(96,2,2,0)], cache: null,         rl: '1000/s', acl: 'allow' },
  { host: 'app.acme.com',                     up: 'next-edge.prod.svc:3000',       cert: 'active',    certDays: 64, rps: 2118, codes: [codes(98,1,1,0),codes(98,1,1,0),codes(97,2,1,0),codes(98,1,1,0),codes(98,1,1,0)], cache: 'HIT 91%',    rl: null,     acl: null },
  { host: 'stripe-webhook-receiver.prod.acme.com', up: 'webhook-relay.prod.svc:9090', cert: 'active',  certDays: 42, rps: 12,   codes: [codes(99,0,1,0),codes(100,0,0,0),codes(98,0,2,0),codes(99,0,1,0),codes(100,0,0,0)], cache: null,    rl: '60/m',   acl: 'allow' },
  { host: 'auth.prod.acme.com',               up: 'keycloak.prod.svc:8080',        cert: 'renewing',  certDays: 9,  rps: 312,  codes: [codes(93,3,4,0),codes(91,3,5,1),codes(90,2,7,1),codes(92,3,5,0),codes(94,2,4,0)], cache: null,         rl: '200/s',  acl: 'allow' },
  { host: 'admin.acme.com',                   up: 'next-admin.prod.svc:3000',      cert: 'active',    certDays: 64, rps: 41,   codes: [codes(99,1,0,0),codes(98,1,1,0),codes(99,0,1,0),codes(98,1,1,0),codes(99,1,0,0)], cache: 'HIT 64%',    rl: '50/s',   acl: 'allow' },
  { host: 'grafana.internal.acme.com',        up: 'grafana.monitoring.svc:3000',   cert: 'active',    certDays: 31, rps: 4,    codes: [codes(100,0,0,0),codes(100,0,0,0),codes(99,0,1,0),codes(100,0,0,0),codes(100,0,0,0)], cache: null,    rl: null,     acl: 'allow' },
  { host: 'prometheus.internal.acme.com',     up: 'prometheus.monitoring.svc:9090',cert: 'active',    certDays: 31, rps: 17,   codes: [codes(100,0,0,0),codes(99,0,1,0),codes(100,0,0,0),codes(99,0,1,0),codes(100,0,0,0)], cache: null,    rl: null,     acl: 'allow' },
  { host: 'registry.internal.acme.com',       up: 'harbor-core.registry.svc:443',  cert: 'active',    certDays: 21, rps: 86,   codes: [codes(96,2,2,0),codes(95,2,3,0),codes(94,2,3,1),codes(96,2,2,0),codes(95,2,3,0)], cache: 'HIT 78%',    rl: null,     acl: 'allow' },
  { host: 'vault.prod.acme.com',              up: 'vault-active.security.svc:8200',cert: 'active',    certDays: 89, rps: 51,   codes: [codes(98,1,1,0),codes(97,2,1,0),codes(98,1,1,0),codes(98,1,1,0),codes(98,1,1,0)], cache: null,         rl: '500/s',  acl: 'allow' },
  { host: 'ci.acme.com',                      up: 'drone-server.ci.svc:80',        cert: 'active',    certDays: 47, rps: 8,    codes: [codes(99,0,1,0),codes(98,0,2,0),codes(99,0,1,0),codes(99,1,0,0),codes(99,0,1,0)], cache: null,         rl: null,     acl: null },
  { host: 'argocd.prod.acme.com',             up: 'argocd-server.argocd.svc:443',  cert: 'active',    certDays: 47, rps: 22,   codes: [codes(98,1,1,0),codes(98,1,1,0),codes(99,0,1,0),codes(98,1,1,0),codes(98,1,1,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'traefik.staging.acme.com',         up: 'traefik-ingress.staging.svc:9000', cert: 'expiring', certDays: 6, rps: 14,  codes: [codes(95,2,3,0),codes(94,2,3,1),codes(93,2,4,1),codes(94,2,3,1),codes(94,2,3,1)], cache: null,        rl: null,     acl: null },
  { host: 's3.internal.acme.com',             up: 'minio-headless.storage.svc:9000',cert: 'active',   certDays: 71, rps: 1284, codes: [codes(97,1,2,0),codes(96,1,3,0),codes(97,1,2,0),codes(96,1,3,0),codes(97,1,2,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'mqtt.iot.acme.com',                up: 'emqx-broker.iot.svc:1883',      cert: 'active',    certDays: 53, rps: 19,   codes: [codes(99,0,1,0),codes(99,0,1,0),codes(99,0,1,0),codes(99,0,1,0),codes(99,0,1,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'nats.internal.acme.com',           up: 'nats-cluster.messaging.svc:4222',cert: 'active',   certDays: 53, rps: 47,   codes: [codes(100,0,0,0),codes(99,0,1,0),codes(100,0,0,0),codes(99,0,1,0),codes(100,0,0,0)], cache: null,    rl: null,     acl: 'allow' },
  { host: 'kafka-rest.prod.acme.com',         up: 'kafka-rest-proxy.streaming.svc:8082', cert: 'active', certDays: 22, rps: 612, codes: [codes(96,2,2,0),codes(95,2,3,0),codes(94,2,3,1),codes(95,2,3,0),codes(95,2,3,0)], cache: null,   rl: '2000/s', acl: 'allow' },
  { host: 'pgadmin.staging.acme.com',         up: 'pgadmin.staging.svc:80',        cert: 'active',    certDays: 14, rps: 1,    codes: [codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0)], cache: null, rl: null,     acl: 'allow' },
  { host: 'sentry.internal.acme.com',         up: 'sentry-web.observability.svc:9000', cert: 'active', certDays: 78, rps: 33,  codes: [codes(98,1,1,0),codes(97,2,1,0),codes(98,1,1,0),codes(98,1,1,0),codes(97,2,1,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'jaeger.internal.acme.com',         up: 'jaeger-query.observability.svc:16686', cert: 'active', certDays: 78, rps: 6, codes: [codes(100,0,0,0),codes(99,0,1,0),codes(100,0,0,0),codes(100,0,0,0),codes(99,0,1,0)], cache: null,   rl: null,     acl: 'allow' },
  { host: 'loki.internal.acme.com',           up: 'loki-gateway.observability.svc:3100', cert: 'active', certDays: 78, rps: 92, codes: [codes(99,0,1,0),codes(98,1,1,0),codes(99,0,1,0),codes(99,1,0,0),codes(99,0,1,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'docs.acme.com',                    up: 'astro-static.docs.svc:80',      cert: 'active',    certDays: 64, rps: 188,  codes: [codes(99,1,0,0),codes(99,0,1,0),codes(99,1,0,0),codes(99,1,0,0),codes(99,0,1,0)], cache: 'HIT 96%',    rl: null,     acl: null },
  { host: 'blog.acme.com',                    up: 'ghost.marketing.svc:2368',      cert: 'active',    certDays: 64, rps: 24,   codes: [codes(99,1,0,0),codes(98,1,1,0),codes(99,1,0,0),codes(99,1,0,0),codes(98,1,1,0)], cache: 'HIT 88%',    rl: null,     acl: null },
  { host: 'status.acme.com',                  up: 'statuspage-static.svc:80',      cert: 'active',    certDays: 64, rps: 9,    codes: [codes(100,0,0,0),codes(100,0,0,0),codes(99,1,0,0),codes(100,0,0,0),codes(100,0,0,0)], cache: 'HIT 99%', rl: null,     acl: null },
  { host: 'webhook.eu.acme.com',              up: 'webhook-eu.relay.svc:9090',     cert: 'pending',   certDays: 0,  rps: 0,    codes: [codes(0,0,0,0),codes(0,0,0,0),codes(0,0,0,0),codes(0,0,0,0),codes(0,0,0,0)], cache: null,                 rl: '60/m',   acl: 'allow' },
  { host: 'legacy-api.acme.com',              up: '10.42.7.18:8080',               cert: 'active',    certDays: 18, rps: 71,   codes: [codes(82,4,12,2),codes(80,4,14,2),codes(78,4,15,3),codes(81,4,12,3),codes(83,3,12,2)], cache: null,    rl: null,     acl: 'deny' },
  { host: 'old-portal.acme.com',              up: 'portal-legacy.deprecated.svc:8080', cert: 'expired', certDays: -3, rps: 0,  codes: [codes(0,0,2,98),codes(0,0,1,99),codes(0,0,3,97),codes(0,0,2,98),codes(0,0,2,98)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'feature-x.dev.acme.com',           up: 'feature-x.dev.svc:3000',        cert: 'active',    certDays: 27, rps: 2,    codes: [codes(95,2,3,0),codes(94,2,4,0),codes(96,2,2,0),codes(95,2,3,0),codes(94,2,4,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'feature-y.dev.acme.com',           up: 'feature-y.dev.svc:3000',        cert: 'active',    certDays: 27, rps: 1,    codes: [codes(96,2,2,0),codes(95,2,3,0),codes(96,2,2,0),codes(95,2,3,0),codes(96,2,2,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'preview-pr-2841.dev.acme.com',     up: '10.42.21.7:3000',               cert: 'active',    certDays: 27, rps: 0,    codes: [codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0)], cache: null,  rl: null,     acl: 'allow' },
  { host: 'preview-pr-2839.dev.acme.com',     up: '10.42.21.4:3000',               cert: 'active',    certDays: 27, rps: 0,    codes: [codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0)], cache: null,  rl: null,     acl: 'allow' },
  { host: 'preview-pr-2837.dev.acme.com',     up: '10.42.21.2:3000',               cert: 'active',    certDays: 27, rps: 0,    codes: [codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0)], cache: null,  rl: null,     acl: 'allow' },
  { host: 'metabase.internal.acme.com',       up: 'metabase.analytics.svc:3000',   cert: 'active',    certDays: 38, rps: 7,    codes: [codes(98,1,1,0),codes(98,1,1,0),codes(97,1,2,0),codes(98,1,1,0),codes(98,1,1,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'airflow.internal.acme.com',        up: 'airflow-webserver.data.svc:8080', cert: 'active',  certDays: 38, rps: 3,    codes: [codes(100,0,0,0),codes(100,0,0,0),codes(99,0,1,0),codes(100,0,0,0),codes(100,0,0,0)], cache: null,    rl: null,     acl: 'allow' },
  { host: 'spark-ui.internal.acme.com',       up: 'spark-history.data.svc:18080',  cert: 'active',    certDays: 38, rps: 1,    codes: [codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0)], cache: null,    rl: null,     acl: 'allow' },
  { host: 'consul.internal.acme.com',         up: 'consul-server.platform.svc:8500', cert: 'active',  certDays: 11, rps: 145,  codes: [codes(99,0,1,0),codes(99,0,1,0),codes(98,1,1,0),codes(99,0,1,0),codes(99,1,0,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'nomad.internal.acme.com',          up: 'nomad-server.platform.svc:4646', cert: 'active',   certDays: 11, rps: 88,   codes: [codes(99,0,1,0),codes(98,1,1,0),codes(99,0,1,0),codes(99,0,1,0),codes(99,0,1,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'mailhog.dev.acme.com',             up: 'mailhog.dev.svc:8025',          cert: 'active',    certDays: 27, rps: 0,    codes: [codes(0,0,0,0),codes(0,0,0,0),codes(0,0,0,0),codes(0,0,0,0),codes(0,0,0,0)], cache: null,                 rl: null,     acl: null },
  { host: 'redis-commander.dev.acme.com',     up: 'redis-commander.dev.svc:8081',  cert: 'active',    certDays: 27, rps: 0,    codes: [codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0),codes(100,0,0,0)], cache: null,  rl: null,     acl: 'allow' },
  { host: 'kibana.observability.acme.com',    up: 'kibana.observability.svc:5601', cert: 'active',    certDays: 78, rps: 19,   codes: [codes(99,0,1,0),codes(99,0,1,0),codes(98,1,1,0),codes(99,0,1,0),codes(99,0,1,0)], cache: null,         rl: null,     acl: 'allow' },
  { host: 'elastic.observability.acme.com',   up: 'elastic-coord.observability.svc:9200', cert: 'renewing', certDays: 4, rps: 287, codes: [codes(98,1,1,0),codes(97,1,2,0),codes(96,1,3,0),codes(97,1,2,0),codes(98,1,1,0)], cache: null, rl: '5000/s', acl: 'allow' },
];

// Pretend total — the table renders 40 but the chrome shows 528.
window.HOST_COUNT_TOTAL = 528;

// Cert lifecycle Gantt fixture for a single host.
// Each lane is one cert *version*. Within a version, segments encode phases.
window.CERT_LIFECYCLE = {
  host: 'api.prod.acme.com',
  // Timeline window: 90 days back → 30 days forward.
  windowStart: -90,
  windowEnd: 30,
  versions: [
    {
      id: 'v3',
      serial: '03:8a:2f:c1:9d:e4',
      issuer: "Let's Encrypt R11",
      san: ['api.prod.acme.com', 'www.api.prod.acme.com'],
      segments: [
        { kind: 'issued',  from: -28, to: -28 },
        { kind: 'active',  from: -28, to: 26 },
        { kind: 'renews',  from: 26,  to: 26 },
        { kind: 'expires', from: 62,  to: 62 },
      ],
    },
    {
      id: 'v2',
      serial: '03:81:7e:44:2c:e1',
      issuer: "Let's Encrypt R11",
      san: ['api.prod.acme.com'],
      segments: [
        { kind: 'issued',   from: -88, to: -88 },
        { kind: 'active',   from: -88, to: -28 },
        { kind: 'replaced', from: -28, to: -28 },
      ],
    },
    {
      id: 'v1',
      serial: '03:7a:c4:08:11:9b',
      issuer: "Let's Encrypt R10",
      san: ['api.prod.acme.com'],
      segments: [
        { kind: 'issued',   from: -180, to: -180 }, // off-window
        { kind: 'active',   from: -180, to: -88 },
        { kind: 'replaced', from: -88,  to: -88 },
      ],
    },
  ],
  // ACME events overlay (hover-revealed in real UI).
  events: [
    { day: -28, type: 'order',   detail: 'new-order acme-v02.api.letsencrypt.org', retries: 0 },
    { day: -28, type: 'auth',    detail: 'http-01 challenge solved', retries: 0 },
    { day: -88, type: 'order',   detail: 'new-order acme-v02.api.letsencrypt.org', retries: 1 },
    { day: -88, type: 'auth',    detail: 'http-01 challenge solved', retries: 1 },
    { day: 26,  type: 'renew',   detail: 'scheduled renewal (30d before expiry)' },
  ],
};

// Drafts queue (Stage / Apply)
window.DRAFTS = [
  { id: 'd-2841', kind: 'host', verb: '+', target: 'webhook.eu.acme.com', author: 'tomas', when: '2m ago' },
  { id: 'd-2842', kind: 'host', verb: '~', target: 'auth.prod.acme.com', author: 'tomas', when: '6m ago' },
  { id: 'd-2843', kind: 'upstream', verb: '~', target: 'webhook-relay.prod.svc:9090', author: 'lukas', when: '14m ago' },
  { id: 'd-2844', kind: 'rate-limit', verb: '+', target: 'api.prod.acme.com', author: 'tomas', when: '22m ago' },
  { id: 'd-2845', kind: 'host', verb: '-', target: 'old-portal.acme.com', author: 'martin', when: '1h ago' },
];

// Cmd-K fixture
window.CMDK = {
  recents: [
    { kind: 'host', label: 'api.prod.acme.com' },
    { kind: 'action', label: 'Force renew · auth.prod.acme.com' },
    { kind: 'host', label: 'auth.prod.acme.com' },
  ],
  actions: [
    { kind: 'action', label: 'Add host…', kbd: 'A' },
    { kind: 'action', label: 'Add upstream…' },
    { kind: 'action', label: 'Force renew cert…' },
    { kind: 'action', label: 'Tail logs for host…', kbd: 'L' },
    { kind: 'action', label: 'Stage & apply drafts', kbd: '⇧⏎' },
    { kind: 'action', label: 'Toggle theme', kbd: '⇧T' },
    { kind: 'action', label: 'Open shortcuts' , kbd: '?' },
  ],
};
