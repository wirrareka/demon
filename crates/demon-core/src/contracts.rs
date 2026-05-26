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
}
