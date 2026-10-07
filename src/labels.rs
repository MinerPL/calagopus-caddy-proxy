//! Docker labels read by [caddy-docker-proxy](https://github.com/lucaslorentz/caddy-docker-proxy).
//!
//! One site uses the keys the proxy documents:
//!
//! ```text
//! caddy: play.example.com
//! caddy.reverse_proxy: {{upstreams 8080}}
//! ```
//!
//! Several sites that share a port stay on those same keys, with the domains
//! comma-separated. Different ports use `caddy_0`, `caddy_1`, and so on.

use indexmap::IndexMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelSource {
    pub domain: String,
    pub port: u16,
}

pub fn upstreams(port: u16) -> String {
    format!("{{{{upstreams {port}}}}}")
}

fn managed_key(key: &str) -> bool {
    if key == "caddy" || key == "caddy.reverse_proxy" {
        return true;
    }
    let Some(rest) = key.strip_prefix("caddy_") else {
        return false;
    };
    let (index, suffix) = rest.split_once('.').unwrap_or((rest, ""));
    !index.is_empty()
        && index.bytes().all(|byte| byte.is_ascii_digit())
        && (suffix.is_empty() || suffix == "reverse_proxy")
}

pub fn planned(routes: &[LabelSource]) -> Result<Vec<(String, String)>, String> {
    if routes.is_empty() {
        return Ok(Vec::new());
    }

    let port = routes[0].port;
    let same_port = routes.iter().all(|route| route.port == port);
    let joined = routes
        .iter()
        .map(|route| route.domain.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    if same_port && joined.chars().count() <= 255 {
        return Ok(vec![
            ("caddy".to_string(), joined),
            ("caddy.reverse_proxy".to_string(), upstreams(port)),
        ]);
    }

    let mut labels = Vec::with_capacity(routes.len() * 2);
    for (index, route) in routes.iter().enumerate() {
        if route.domain.chars().count() > 255 {
            return Err(format!(
                "`{}` is too long for a container label",
                route.domain
            ));
        }
        labels.push((format!("caddy_{index}"), route.domain.clone()));
        labels.push((
            format!("caddy_{index}.reverse_proxy"),
            upstreams(route.port),
        ));
    }
    Ok(labels)
}

pub fn merge<K, V>(
    existing: &IndexMap<K, V>,
    routes: &[LabelSource],
) -> Result<Vec<(String, String)>, String>
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut labels = Vec::new();
    for (key, value) in existing {
        if !managed_key(key.as_ref()) {
            labels.push((key.as_ref().to_string(), value.as_ref().to_string()));
        }
    }
    labels.extend(planned(routes)?);
    if labels.len() > 64 {
        return Err("adding this domain would exceed the 64 container label limit".to_string());
    }
    Ok(labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn one_domain_uses_the_docker_proxy_labels() {
        let labels = planned(&[LabelSource {
            domain: "play.example.com".into(),
            port: 25565,
        }])
        .unwrap();

        assert_eq!(
            labels,
            vec![
                ("caddy".into(), "play.example.com".into()),
                ("caddy.reverse_proxy".into(), "{{upstreams 25565}}".into()),
            ]
        );
    }

    #[test]
    fn shared_port_joins_domains() {
        let labels = planned(&[
            LabelSource {
                domain: "a.example.com".into(),
                port: 8080,
            },
            LabelSource {
                domain: "b.example.com".into(),
                port: 8080,
            },
        ])
        .unwrap();

        assert_eq!(labels[0].1, "a.example.com, b.example.com");
        assert_eq!(labels[1].1, "{{upstreams 8080}}");
    }

    #[test]
    fn different_ports_are_numbered() {
        let labels = planned(&[
            LabelSource {
                domain: "a.example.com".into(),
                port: 80,
            },
            LabelSource {
                domain: "b.example.com".into(),
                port: 8080,
            },
        ])
        .unwrap();

        assert_eq!(
            labels,
            vec![
                ("caddy_0".into(), "a.example.com".into()),
                ("caddy_0.reverse_proxy".into(), "{{upstreams 80}}".into()),
                ("caddy_1".into(), "b.example.com".into()),
                ("caddy_1.reverse_proxy".into(), "{{upstreams 8080}}".into()),
            ]
        );
    }

    #[test]
    fn merge_replaces_only_labels_this_extension_owns() {
        let existing = map(&[
            ("caddy", "old.example.com"),
            ("caddy.reverse_proxy", "{{upstreams 1}}"),
            ("caddy_0", "stale.example.com"),
            ("caddy.encode", "zstd"),
            ("com.example.tier", "gold"),
        ]);
        let merged = merge(
            &existing,
            &[LabelSource {
                domain: "play.example.com".into(),
                port: 8080,
            }],
        )
        .unwrap();

        assert!(merged
            .iter()
            .any(|(key, value)| key == "caddy.encode" && value == "zstd"));
        assert!(merged
            .iter()
            .any(|(key, value)| key == "com.example.tier" && value == "gold"));
        assert!(merged
            .iter()
            .any(|(key, value)| key == "caddy" && value == "play.example.com"));
        assert!(!merged.iter().any(|(key, _)| key == "caddy_0"));
    }
}
