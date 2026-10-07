//! Hostname and DNS-target checks shared by the API and the admin form.

use std::net::Ipv4Addr;

const RESERVED_LABELS: &[&str] = &[
    "www",
    "mail",
    "ftp",
    "panel",
    "admin",
    "api",
    "ns",
    "ns1",
    "ns2",
    "caddy",
    "wings",
    "node",
    "localhost",
    "autoconfig",
    "autodiscover",
];

pub fn valid_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    if bytes.is_empty() || bytes.len() > 63 {
        return false;
    }
    let first = bytes[0];
    let last = bytes[bytes.len() - 1];
    if !first.is_ascii_alphanumeric() || !last.is_ascii_alphanumeric() {
        return false;
    }
    bytes
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
}

/// A DNS hostname: lowercase, no trailing dot, at least two labels.
pub fn normalize_hostname(input: &str) -> Option<String> {
    let value = input.trim().trim_end_matches('.').to_ascii_lowercase();
    if value.is_empty() || value.len() > 253 || value.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    let labels: Vec<&str> = value.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|label| !valid_label(label)) {
        return None;
    }
    Some(value)
}

pub fn normalize_ipv4(input: &str) -> Option<String> {
    let ip: Ipv4Addr = input.trim().parse().ok()?;
    if ip.is_unspecified() || ip.is_broadcast() {
        return None;
    }
    Some(ip.to_string())
}

pub fn normalize_record(record_type: &str, content: &str) -> Result<(String, String), String> {
    match record_type {
        "A" => {
            let ip = normalize_ipv4(content)
                .ok_or_else(|| format!("`{content}` is not an IPv4 address"))?;
            Ok(("A".to_string(), ip))
        }
        "CNAME" => {
            let host = normalize_hostname(content)
                .ok_or_else(|| format!("`{content}` is not a hostname"))?;
            Ok(("CNAME".to_string(), host))
        }
        other => Err(format!("record type `{other}` must be A or CNAME")),
    }
}

/// Subdomain relative to an allowed domain: `play` or `mc.play`, never the apex.
pub fn normalize_subdomain_name(input: &str) -> Result<String, String> {
    let value = input.trim().trim_end_matches('.').to_ascii_lowercase();
    if value.is_empty() {
        return Err("enter a subdomain name".to_string());
    }
    if value.contains('_') || value.parse::<std::net::IpAddr>().is_ok() {
        return Err("that subdomain name is not valid".to_string());
    }
    let labels: Vec<&str> = value.split('.').collect();
    if labels.iter().any(|label| !valid_label(label)) {
        return Err("use letters, numbers and dashes only".to_string());
    }
    if labels.iter().any(|label| RESERVED_LABELS.contains(label)) {
        return Err("that subdomain name is reserved".to_string());
    }
    Ok(value)
}

pub fn join_domain(name: &str, parent: &str) -> Result<String, String> {
    let fqdn = format!("{name}.{parent}");
    if fqdn.len() > 253 {
        return Err("that subdomain is too long".to_string());
    }
    normalize_hostname(&fqdn).ok_or_else(|| "that subdomain is not a valid hostname".to_string())
}

pub fn is_under_domain(fqdn: &str, parent: &str) -> bool {
    fqdn == parent || fqdn.ends_with(&format!(".{parent}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostnames_are_normalized() {
        assert_eq!(
            normalize_hostname(" Play.Example.COM. "),
            Some("play.example.com".to_string())
        );
        assert!(normalize_hostname("localhost").is_none());
        assert!(normalize_hostname("1.2.3.4").is_none());
        assert!(normalize_hostname("-bad.example.com").is_none());
    }

    #[test]
    fn records_match_their_type() {
        assert_eq!(
            normalize_record("A", "203.0.113.10").unwrap(),
            ("A".into(), "203.0.113.10".into())
        );
        assert!(normalize_record("A", "0.0.0.0").is_err());
        assert_eq!(
            normalize_record("CNAME", "edge.example.net.").unwrap(),
            ("CNAME".into(), "edge.example.net".into())
        );
        assert!(normalize_record("CNAME", "203.0.113.10").is_err());
    }

    #[test]
    fn reserved_names_are_rejected() {
        assert!(normalize_subdomain_name("www").is_err());
        assert_eq!(normalize_subdomain_name("Play").unwrap(), "play");
        assert_eq!(normalize_subdomain_name("mc.play").unwrap(), "mc.play");
    }

    #[test]
    fn subdomain_joins_its_parent() {
        assert_eq!(
            join_domain("play", "example.com").unwrap(),
            "play.example.com"
        );
        assert_eq!(
            join_domain("mc.play", "example.com").unwrap(),
            "mc.play.example.com"
        );
        assert!(is_under_domain("play.example.com", "example.com"));
        assert!(is_under_domain("example.com", "example.com"));
        assert!(!is_under_domain("example.com.evil.test", "example.com"));
        assert!(!is_under_domain("notexample.com", "example.com"));
    }
}
