//! Read-only host **line-contract** parsers, lifted from proximiio-tui.
//!
//! Each host-side `check-<area>.sh` script emits a stable, tab-separated,
//! machine-parseable contract: one record per line, the first token a record tag,
//! the rest `key=value` pairs. Exit code is always 0 — status lives in the line.
//!
//! Parsing is **pure and never panics**: malformed or empty input degrades to a
//! graceful `Unknown`/empty value (the non-negotiable from the design docs).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::health::HealthStatus;

/// Parse one tab-separated contract line into `(tag, fields)`.
///
/// The first tab-separated token is the record tag; every remaining `key=value`
/// token becomes a map entry (tokens without `=` are ignored). Returns `None` for a
/// blank line.
#[must_use]
pub fn parse_line(line: &str) -> Option<(&str, BTreeMap<&str, &str>)> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.trim().is_empty() {
        return None;
    }
    let mut parts = line.split('\t');
    let tag = parts.next()?;
    let mut fields = BTreeMap::new();
    for token in parts {
        if let Some((k, v)) = token.split_once('=') {
            fields.insert(k, v);
        }
    }
    Some((tag, fields))
}

/// Find the first line whose record tag equals `tag` and return its fields.
#[must_use]
pub fn find_record<'a>(output: &'a str, tag: &str) -> Option<BTreeMap<&'a str, &'a str>> {
    output
        .lines()
        .filter_map(parse_line)
        .find(|(t, _)| *t == tag)
        .map(|(_, fields)| fields)
}

/// OS family detected on a host (`check-os.sh`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OsFamily {
    FreeBsd,
    Linux,
    /// Anything unrecognised or absent.
    #[default]
    Unknown,
}

impl OsFamily {
    fn parse(s: &str) -> Self {
        match s {
            "freebsd" => OsFamily::FreeBsd,
            "linux" => OsFamily::Linux,
            _ => OsFamily::Unknown,
        }
    }
}

/// Parsed OS / platform report from `check-os.sh`'s `OS` line contract:
/// `OS\thost=..\tfamily=..\tid=..\tversion=..\tpkg=..\tservice=..\tfirewall=..\tcontainer=..`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OsStatus {
    /// Reported hostname (empty when unknown).
    pub host: String,
    /// OS family.
    pub family: OsFamily,
    /// Distro/OS id (`freebsd`, `debian`, `ubuntu`, ...); empty when unknown.
    pub id: String,
    /// OS version string; empty when unknown.
    pub version: String,
    /// Package backend (`pkg` | `apt` | `dnf`); empty when unknown.
    pub pkg: String,
    /// Service manager (`rc` | `systemd`); empty when unknown.
    pub service: String,
    /// Firewall backend (`pf` | `nftables` | `iptables` | `unknown`).
    pub firewall: String,
    /// Container runtime (`jail` | `podman` | `docker` | `none`); empty when unknown.
    pub container: String,
}

impl OsStatus {
    /// Reachability/detection rollup: `Up` if the OS family was detected, else
    /// `Unknown` (host unreachable or contract unparseable). OS is informational, so
    /// it never reports `Degraded`/`Down` itself.
    #[must_use]
    pub fn health(&self) -> HealthStatus {
        if self.family == OsFamily::Unknown {
            HealthStatus::Unknown
        } else {
            HealthStatus::Up
        }
    }
}

/// Parse `check-os.sh` output into an [`OsStatus`]. Missing/malformed output yields a
/// default (`Unknown` family, empty fields) — never panics.
#[must_use]
pub fn parse_os(output: &str) -> OsStatus {
    let Some(f) = find_record(output, "OS") else {
        return OsStatus::default();
    };
    let get = |k: &str| f.get(k).map_or_else(String::new, |s| (*s).to_owned());
    OsStatus {
        host: get("host"),
        family: f.get("family").map_or(OsFamily::Unknown, |s| OsFamily::parse(s)),
        id: get("id"),
        version: get("version"),
        pkg: get("pkg"),
        service: get("service"),
        firewall: get("firewall"),
        container: get("container"),
    }
}

/// Backup-recency posture from `check-backup.sh`'s `BACKUP` summary line:
/// `BACKUP\thost=..\tstores=..\tworst_age_hours=..\tverdict=<ok|stale|unknown>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BackupStatus {
    /// Reported hostname.
    pub host: String,
    /// Number of backup stores discovered for this host.
    pub stores: u32,
    /// Age (hours) of the oldest most-recent-successful backup; `None` if unknown.
    pub worst_age_hours: Option<u64>,
    /// Summary verdict (`ok` | `stale` | `unknown`).
    pub verdict: String,
}

impl BackupStatus {
    /// Roll the verdict into a [`HealthStatus`].
    #[must_use]
    pub fn health(&self) -> HealthStatus {
        match self.verdict.as_str() {
            "ok" => HealthStatus::Up,
            "stale" => HealthStatus::Degraded,
            _ => HealthStatus::Unknown,
        }
    }
}

/// Parse `check-backup.sh` output. Missing/malformed yields default (`unknown`).
#[must_use]
pub fn parse_backup(output: &str) -> BackupStatus {
    let Some(f) = find_record(output, "BACKUP") else {
        return BackupStatus::default();
    };
    BackupStatus {
        host: f.get("host").map_or_else(String::new, |s| (*s).to_owned()),
        stores: f.get("stores").and_then(|s| s.parse().ok()).unwrap_or(0),
        worst_age_hours: f.get("worst_age_hours").and_then(|s| s.parse().ok()),
        verdict: f
            .get("verdict")
            .map_or_else(String::new, |s| (*s).to_owned()),
    }
}

/// File-integrity posture from `check-fim.sh`'s `FIM` line:
/// `FIM\thost=..\tlast_verify=..\tdrift=..\tpkg_mismatch=..\tbaseline=<present|partial|missing>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FimStatus {
    /// Reported hostname.
    pub host: String,
    /// Epoch of the last fim-verify run (0 if never/unknown).
    pub last_verify: i64,
    /// Total drift entries this probe found.
    pub drift: u32,
    /// Files failing package verification.
    pub pkg_mismatch: u32,
    /// Baseline coverage (`present` | `partial` | `missing`).
    pub baseline: String,
}

impl FimStatus {
    /// Roll the integrity posture into a [`HealthStatus`]: missing baseline is `Down`;
    /// a complete clean baseline is `Up`; anything else (drift, mismatches, partial
    /// baseline) is `Degraded`.
    #[must_use]
    pub fn health(&self) -> HealthStatus {
        match self.baseline.as_str() {
            "missing" => HealthStatus::Down,
            "present" if self.drift == 0 && self.pkg_mismatch == 0 => HealthStatus::Up,
            "present" | "partial" => HealthStatus::Degraded,
            _ => HealthStatus::Unknown,
        }
    }
}

/// Parse `check-fim.sh` output. Missing/malformed yields default (`Unknown`).
#[must_use]
pub fn parse_fim(output: &str) -> FimStatus {
    let Some(f) = find_record(output, "FIM") else {
        return FimStatus::default();
    };
    FimStatus {
        host: f.get("host").map_or_else(String::new, |s| (*s).to_owned()),
        last_verify: f.get("last_verify").and_then(|s| s.parse().ok()).unwrap_or(0),
        drift: f.get("drift").and_then(|s| s.parse().ok()).unwrap_or(0),
        pkg_mismatch: f
            .get("pkg_mismatch")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        baseline: f
            .get("baseline")
            .map_or_else(String::new, |s| (*s).to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str =
        "OS\thost=core-1\tfamily=freebsd\tid=freebsd\tversion=14.1-RELEASE\tpkg=pkg\tservice=rc\tfirewall=pf\tcontainer=jail";

    #[test]
    fn parses_freebsd_os_line() {
        let os = parse_os(SAMPLE);
        assert_eq!(os.host, "core-1");
        assert_eq!(os.family, OsFamily::FreeBsd);
        assert_eq!(os.version, "14.1-RELEASE");
        assert_eq!(os.firewall, "pf");
        assert_eq!(os.container, "jail");
    }

    #[test]
    fn parses_linux_among_other_lines() {
        let out = "NOISE\tx=1\nOS\thost=t-9\tfamily=linux\tid=ubuntu\tversion=22.04\tpkg=apt\tservice=systemd\tfirewall=nftables\tcontainer=docker\n";
        let os = parse_os(out);
        assert_eq!(os.family, OsFamily::Linux);
        assert_eq!(os.id, "ubuntu");
        assert_eq!(os.service, "systemd");
    }

    #[test]
    fn empty_or_malformed_degrades_to_unknown() {
        assert_eq!(parse_os("").family, OsFamily::Unknown);
        assert_eq!(parse_os("garbage without tabs").family, OsFamily::Unknown);
        // OS line present but family unrecognised -> Unknown, other fields still read.
        let os = parse_os("OS\thost=h\tfamily=plan9\tid=x");
        assert_eq!(os.family, OsFamily::Unknown);
        assert_eq!(os.host, "h");
        assert_eq!(os.id, "x");
    }

    #[test]
    fn parse_line_blank_is_none() {
        assert!(parse_line("").is_none());
        assert!(parse_line("   \n").is_none());
        let (tag, f) = parse_line("OS\tk=v\tnoeq\tk2=v2").unwrap();
        assert_eq!(tag, "OS");
        assert_eq!(f.get("k"), Some(&"v"));
        assert_eq!(f.get("k2"), Some(&"v2"));
        assert!(!f.contains_key("noeq"));
    }

    #[test]
    fn backup_verdict_to_health() {
        let ok = parse_backup("BACKUP\thost=h\tstores=3\tworst_age_hours=6\tverdict=ok");
        assert_eq!(ok.stores, 3);
        assert_eq!(ok.worst_age_hours, Some(6));
        assert_eq!(ok.health(), HealthStatus::Up);
        let stale = parse_backup("BACKUP\thost=h\tstores=2\tworst_age_hours=99\tverdict=stale");
        assert_eq!(stale.health(), HealthStatus::Degraded);
        let unk = parse_backup("BACKUP\thost=h\tstores=0\tworst_age_hours=unknown\tverdict=unknown");
        assert_eq!(unk.worst_age_hours, None);
        assert_eq!(unk.health(), HealthStatus::Unknown);
        assert_eq!(parse_backup("").health(), HealthStatus::Unknown);
    }

    #[test]
    fn fim_baseline_and_drift_to_health() {
        let clean = parse_fim("FIM\thost=h\tlast_verify=1700\tdrift=0\tpkg_mismatch=0\tbaseline=present");
        assert_eq!(clean.last_verify, 1700);
        assert_eq!(clean.health(), HealthStatus::Up);
        let drifted = parse_fim("FIM\thost=h\tlast_verify=1700\tdrift=4\tpkg_mismatch=0\tbaseline=present");
        assert_eq!(drifted.drift, 4);
        assert_eq!(drifted.health(), HealthStatus::Degraded);
        let partial = parse_fim("FIM\thost=h\tlast_verify=0\tdrift=0\tpkg_mismatch=0\tbaseline=partial");
        assert_eq!(partial.health(), HealthStatus::Degraded);
        let missing = parse_fim("FIM\thost=h\tlast_verify=0\tdrift=0\tpkg_mismatch=0\tbaseline=missing");
        assert_eq!(missing.health(), HealthStatus::Down);
        assert_eq!(parse_fim("").health(), HealthStatus::Unknown);
    }
}
