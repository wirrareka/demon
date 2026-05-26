//! `demon-tui` — the terminal thin client.
//!
//! Phase 1 deliberately stays minimal: it exists to **prove the API**. It connects to
//! a running daemon over the (WireGuard) REST API via [`demon_sdk::Client`], prints the
//! fleet inventory and each host's current per-area health, and exits. It holds **no
//! business logic** — every value rendered comes straight from `demon-core` view models
//! served by the daemon. The full interactive ratatui client (refactored from
//! proximiio-tui to render server-pushed state) arrives in a later increment.
#![forbid(unsafe_code)]

use std::fmt::Write as _;

use demon_sdk::{Client, HealthSnapshot, Host};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base = std::env::var("DEMON_API").unwrap_or_else(|_| "http://127.0.0.1:8787".to_owned());
    let client = Client::new(&base);

    let version = client.version().await?;
    println!(
        "connected to {} v{} [{}] @ {base}",
        version.service, version.version, version.region
    );

    let hosts = client.hosts().await?;
    println!("\n{}", format_hosts(&hosts));

    for host in &hosts {
        let health = client.host_health(&host.id).await?;
        println!("\n{} ({}):", host.fqdn, host.id);
        println!("{}", format_health(&health));
    }
    Ok(())
}

/// Render the host inventory as an aligned text table. Pure — unit-tested.
#[must_use]
#[allow(clippy::write_literal)] // column-header labels are intentionally literal
fn format_hosts(hosts: &[Host]) -> String {
    if hosts.is_empty() {
        return "(no hosts enrolled)".to_owned();
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<14}  {:<28}  {:<7}  {:<10}  {}",
        "ID", "FQDN", "FLEET", "OS", "WG_IP"
    );
    for h in hosts {
        let _ = writeln!(
            out,
            "{:<14}  {:<28}  {:<7}  {:<10}  {}",
            h.id,
            h.fqdn,
            h.fleet.as_str(),
            h.os,
            h.wg_ip.as_deref().unwrap_or("-"),
        );
    }
    out
}

/// Render a host's per-area health as aligned `area: status` lines. Pure.
#[must_use]
fn format_health(snapshots: &[HealthSnapshot]) -> String {
    if snapshots.is_empty() {
        return "  (no health observed yet)".to_owned();
    }
    let mut out = String::new();
    for s in snapshots {
        let _ = writeln!(out, "  {:<12} {}", s.area, s.status.as_str());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use demon_sdk::{Fleet, HealthStatus, Region, TargetKind};

    #[test]
    fn empty_inventory_message() {
        assert_eq!(format_hosts(&[]), "(no hosts enrolled)");
        assert!(format_health(&[]).contains("no health observed"));
    }

    #[test]
    fn renders_host_and_health_rows() {
        let hosts = vec![Host {
            id: "h1".into(),
            fqdn: "core-1.eu".into(),
            fleet: Fleet::Core,
            os: "freebsd".into(),
            residency_group: Region::Eu,
            wg_ip: Some("10.200.0.5".into()),
            tenant_id: None,
            enrolled_at: 1,
            last_seen: None,
        }];
        let table = format_hosts(&hosts);
        assert!(table.contains("core-1.eu"));
        assert!(table.contains("10.200.0.5"));

        let health = vec![HealthSnapshot {
            target_id: "h1".into(),
            target_kind: TargetKind::Host,
            area: "os".into(),
            status: HealthStatus::Up,
            raw_json: "{}".into(),
            observed_at: 1,
        }];
        let rendered = format_health(&health);
        assert!(rendered.contains("os"));
        assert!(rendered.contains("up"));
    }
}
