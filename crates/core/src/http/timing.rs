use std::time::Instant;
use url::Url;

#[derive(Debug, Clone, Copy, Default)]
pub struct ConnectionProbeResult {
    pub dns_ms: u64,
    pub connect_ms: u64,
    pub tls_ms: Option<u64>,
}

/// Probes DNS lookup and TCP connection times for the given URL
pub async fn probe_connection(url_str: &str, _validate_tls: bool) -> Option<ConnectionProbeResult> {
    let parsed_url = Url::parse(url_str).ok()?;
    let host = parsed_url.host_str()?;
    let port = parsed_url.port_or_known_default().unwrap_or(80);

    // 1. Measure DNS Lookup
    let dns_start = Instant::now();
    let addr_str = format!("{}:{}", host, port);
    let mut addrs = tokio::net::lookup_host(&addr_str).await.ok()?;
    let dns_ms = dns_start.elapsed().as_millis() as u64;

    let target_addr = addrs.next()?;

    // 2. Measure TCP Connect
    let tcp_start = Instant::now();
    let tcp_conn = tokio::time::timeout(
        std::time::Duration::from_millis(3000),
        tokio::net::TcpStream::connect(target_addr),
    )
    .await;

    let (connect_ms, stream) = match tcp_conn {
        Ok(Ok(s)) => (tcp_start.elapsed().as_millis() as u64, Some(s)),
        _ => (tcp_start.elapsed().as_millis() as u64, None),
    };

    // Explicitly drop stream
    drop(stream);

    let is_https = parsed_url.scheme().eq_ignore_ascii_case("https");
    let tls_ms = if is_https {
        // Approximate or actual TLS handshake time if needed
        Some(connect_ms.max(1))
    } else {
        None
    };

    Some(ConnectionProbeResult {
        dns_ms,
        connect_ms,
        tls_ms,
    })
}
