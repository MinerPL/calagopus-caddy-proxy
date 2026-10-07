//! The public name of a Wings node, used as the default CNAME target.

use crate::domain::normalize_hostname;
use shared::models::node::Node;

/// Hostname from the node's public URL, then its panel URL, then its SFTP host.
pub fn hostname(node: &Node) -> Option<String> {
    node.public_url
        .as_ref()
        .and_then(|url| url.host_str())
        .and_then(normalize_hostname)
        .or_else(|| node.url.host_str().and_then(normalize_hostname))
        .or_else(|| node.sftp_host.as_deref().and_then(normalize_hostname))
}

/// Default public record: a CNAME to the node's own domain.
pub fn default_record(node: &Node) -> Option<(String, String)> {
    hostname(node).map(|host| ("CNAME".to_string(), host))
}
