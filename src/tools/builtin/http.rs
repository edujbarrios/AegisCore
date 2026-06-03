use crate::registry::{Tool, ToolContext, ToolHandler, ToolRegistry};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

pub fn register(registry: &mut ToolRegistry) -> anyhow::Result<()> {
    registry.register(Tool {
        name: "http_get".to_string(),
        description: "Perform an HTTP GET request with localhost/private network protection"
            .to_string(),
        parameters: serde_json::json!({
            "type":"object",
            "additionalProperties": false,
            "required": ["url"],
            "properties": {
                "url": {"type":"string"},
                "headers": {"type":"object","additionalProperties":{"type":"string"}},
                "timeout_ms": {"type":"integer","minimum":1,"maximum":60000},
                "max_bytes": {"type":"integer","minimum":1}
            }
        }),
        handler: Arc::new(HttpGetTool),
    })?;
    Ok(())
}

struct HttpGetTool;

#[async_trait]
impl ToolHandler for HttpGetTool {
    async fn call(&self, args: Value, ctx: ToolContext) -> anyhow::Result<Value> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing url"))?;
        let timeout_ms = args
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(ctx.http_timeout_ms)
            .min(60_000);
        let max_bytes = args
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(ctx.http_max_bytes)
            .min(ctx.http_max_bytes);

        let parsed = reqwest::Url::parse(url)?;

        let mut headers = HeaderMap::new();
        if let Some(map) = args.get("headers").and_then(|v| v.as_object()) {
            for (k, v) in map {
                let name = HeaderName::from_bytes(k.as_bytes())?;
                let value = HeaderValue::from_str(v.as_str().unwrap_or_default())?;
                headers.insert(name, value);
            }
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        const MAX_REDIRECTS: usize = 5;
        let mut current = parsed;
        for step in 0..=MAX_REDIRECTS {
            enforce_http_url_policy(&current).await?;

            let resp = client
                .get(current.clone())
                .headers(headers.clone())
                .send()
                .await?;

            if resp.status().is_redirection() {
                if step == MAX_REDIRECTS {
                    anyhow::bail!("too many redirects");
                }
                let loc = resp
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| anyhow::anyhow!("redirect missing Location header"))?;
                current = current.join(loc)?;
                continue;
            }

            let resp = resp.error_for_status()?;
            let status = resp.status().as_u16();
            let bytes = resp.bytes().await?;
            let truncated = (bytes.len() as u64) > max_bytes;
            let limited = if truncated {
                bytes.slice(0..(max_bytes as usize))
            } else {
                bytes
            };
            let body = String::from_utf8_lossy(&limited).to_string();
            return Ok(serde_json::json!({
                "status": status,
                "truncated": truncated,
                "body": body
            }));
        }

        anyhow::bail!("unreachable")
    }
}

async fn enforce_http_url_policy(url: &reqwest::Url) -> anyhow::Result<()> {
    match url.scheme() {
        "http" | "https" => {}
        _ => anyhow::bail!("unsupported scheme"),
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("url missing host"))?;
    let host_lc = host.to_ascii_lowercase();
    if host_lc == "localhost" || host_lc.ends_with(".localhost") || host_lc.ends_with(".local") {
        anyhow::bail!("localhost domains are blocked");
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("url missing port"))?;

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            anyhow::bail!("private/loopback/link-local IPs are blocked");
        }
        return Ok(());
    }

    let addrs = tokio::net::lookup_host((host, port)).await?;
    let mut saw_any = false;
    for addr in addrs {
        saw_any = true;
        if is_blocked_ip(addr.ip()) {
            anyhow::bail!("private/loopback/link-local IPs are blocked");
        }
    }
    if !saw_any {
        anyhow::bail!("host did not resolve to any IPs");
    }

    Ok(())
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_unspecified()
                || v4 == Ipv4Addr::BROADCAST
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_multicast()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || v6 == Ipv6Addr::LOCALHOST
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_private_ipv4() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 2))));
    }

    #[tokio::test]
    async fn blocks_localhost_domain() {
        let url = reqwest::Url::parse("http://localhost:1234/").unwrap();
        let err = enforce_http_url_policy(&url).await.unwrap_err();
        assert!(err.to_string().contains("localhost domains are blocked"));
    }

    #[tokio::test]
    async fn blocks_loopback_ip() {
        let url = reqwest::Url::parse("http://127.0.0.1:1234/").unwrap();
        let err = enforce_http_url_policy(&url).await.unwrap_err();
        assert!(err
            .to_string()
            .contains("private/loopback/link-local IPs are blocked"));
    }
}
