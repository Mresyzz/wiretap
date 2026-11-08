use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScannedPort {
    pub port: u16,
    pub is_open: bool,
    pub is_http: bool,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScanQuery {
    pub mode: Option<String>, // "dev", "all", or "custom"
    pub start: Option<u16>,   // for custom range
    pub end: Option<u16>,     // for custom range
}

/// Probes a single TCP port on localhost (127.0.0.1).
/// Sends a lightweight HTTP request if the port is open to detect HTTP services.
pub async fn probe_port(port: u16, check_http: bool) -> Option<ScannedPort> {
    let addr_str = format!("127.0.0.1:{}", port);
    let addr: SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(_) => return None,
    };

    // On localhost, closed ports get RST instantly (in microseconds).
    // An active listener will connect within a couple of milliseconds.
    let connect_res = timeout(Duration::from_millis(150), TcpStream::connect(addr)).await;
    match connect_res {
        Ok(Ok(mut stream)) => {
            let mut is_http = false;
            let mut banner = None;

            if check_http {
                let req = format!(
                    "GET / HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nUser-Agent: WireTap-PortScanner/1.0\r\nAccept: */*\r\nConnection: close\r\n\r\n",
                    port
                );
                let _ = stream.write_all(req.as_bytes()).await;
                let mut buf = [0u8; 512];
                if let Ok(Ok(n)) = timeout(Duration::from_millis(150), stream.read(&mut buf)).await {
                    if n > 0 {
                        let resp = String::from_utf8_lossy(&buf[..n]);
                        if resp.starts_with("HTTP/") {
                            is_http = true;
                            if let Some(first_line) = resp.lines().next() {
                                banner = Some(first_line.trim().to_string());
                            }
                        }
                    }
                }
            }

            Some(ScannedPort {
                port,
                is_open: true,
                is_http,
                banner,
            })
        }
        _ => None,
    }
}

/// Common web & API developer service ports
pub fn get_dev_ports() -> Vec<u16> {
    let mut ports = vec![
        80, 443, 3000, 3001, 3002, 3333, 4000, 4200, 5000, 5001, 5173, 5174, 5175, 5500, 6006,
        7000, 7001, 8000, 8001, 8080, 8081, 8082, 8088, 8888, 9000, 9001, 9090, 9999, 10000,
    ];
    ports.sort_unstable();
    ports.dedup();
    ports
}

/// Concurrently scans a list of ports with bounded concurrency
pub async fn scan_ports_list(ports: Vec<u16>, concurrency: usize) -> Vec<ScannedPort> {
    let mut results: Vec<ScannedPort> = stream::iter(ports)
        .map(|port| async move { probe_port(port, true).await })
        .buffer_unordered(concurrency)
        .filter_map(|res| async move { res })
        .collect()
        .await;

    results.sort_by_key(|p| p.port);
    results
}

/// Scans all ports from 1 to 65535 or custom range
pub async fn scan_ports_range(start: u16, end: u16, concurrency: usize) -> Vec<ScannedPort> {
    let range: Vec<u16> = (start..=end).collect();
    scan_ports_list(range, concurrency).await
}
