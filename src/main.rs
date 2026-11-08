use std::sync::Arc;
use tokio::sync::RwLock;
use clap::{Parser, Subcommand};
use colored::*;

use wiretap::config::{WireTapConfig, DEFAULT_CONFIG_FILE};
use wiretap::demo;
use wiretap::model;
use wiretap::proxy::{build_ca, start_hudsucker_proxy};
use wiretap::store::TrafficStore;
use wiretap::web::{create_web_router, WebState};

#[derive(Parser, Debug)]
#[command(
    name = "wiretap",
    author = "WireTap Maintainers",
    version = "0.1.0",
    about = "Zero-dependency, single-binary API traffic inspector and replay studio",
    long_about = "WireTap intercepts, inspects, and replays HTTP API traffic in real-time with an embedded zero-config web studio. A local-first, privacy-respecting alternative to bloated SaaS tools."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Listening port for the proxy (e.g. :8080 or 8080)
    #[arg(short, long, default_value = "8080")]
    listen: String,

    /// Upstream target base URL to proxy to (e.g. http://localhost:3000)
    #[arg(short, long)]
    to: Option<String>,

    /// Port for the embedded Web Studio UI
    #[arg(long, default_value = "9090")]
    ui_port: u16,

    /// Maximum number of traffic items to retain in memory
    #[arg(long, default_value = "1000")]
    max_history: usize,

    /// Automatically configure OS system proxy on launch and restore on exit
    #[arg(short = 's', long)]
    sys: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch the zero-setup demo mode with live simulated traffic
    Demo {
        /// Port for the embedded Web Studio UI
        #[arg(long, default_value = "9090")]
        port: u16,
    },
    /// Intercept and inspect traffic by proxying to an upstream service
    Proxy {
        /// Listening port (e.g. 8080 or :8080)
        #[arg(short, long, default_value = "8080")]
        listen: String,

        /// Target base URL to forward requests to (e.g. http://localhost:3000)
        #[arg(short, long)]
        target: String,

        /// Port for the embedded Web Studio UI
        #[arg(long, default_value = "9090")]
        ui_port: u16,

        /// Automatically configure OS system proxy on launch and restore on exit
        #[arg(short = 's', long)]
        sys: bool,
    },
    /// Run as a standalone mock/sink server to inspect webhooks or incoming requests
    Mock {
        /// Listening port (e.g. 8080 or :8080)
        #[arg(default_value = "8080")]
        listen: String,

        /// Port for the embedded Web Studio UI
        #[arg(long, default_value = "9090")]
        ui_port: u16,
    },
}

#[tokio::main]
async fn main() {
    // 智能参数预处理：支持开发者直觉敲法 `wiretap :8080 --to ...` 或 `wiretap 8080 ...`
    let raw_args: Vec<String> = std::env::args().collect();
    let mut normalized_args = Vec::new();
    let mut i = 0;

    while i < raw_args.len() {
        if i == 0 {
            normalized_args.push(raw_args[0].clone());
            i += 1;
            continue;
        }

        let arg = &raw_args[i];
        let is_subcmd = matches!(arg.as_str(), "demo" | "proxy" | "mock" | "help" | "--help" | "-h" | "-V" | "--version");
        let prev_is_option = i > 0 && raw_args[i - 1].starts_with('-');
        let has_subcmd_before = raw_args[1..i].iter().any(|a| matches!(a.as_str(), "demo" | "proxy" | "mock"));
        let is_port_syntax = !prev_is_option && !has_subcmd_before && (
            (arg.starts_with(':') && arg.len() > 1 && arg[1..].chars().all(|c| c.is_ascii_digit()))
            || (!arg.starts_with('-') && !is_subcmd && !arg.is_empty() && arg.chars().all(|c| c.is_ascii_digit()))
        );

        if is_port_syntax && !is_subcmd {
            normalized_args.push("--listen".to_string());
            normalized_args.push(arg.trim_start_matches(':').to_string());
        } else {
            normalized_args.push(arg.clone());
        }
        i += 1;
    }

    let cli = match Cli::try_parse_from(&normalized_args) {
        Ok(c) => c,
        Err(err) => {
            err.exit();
        }
    };

    match cli.command {
        Some(Commands::Demo { port }) => {
            demo::run_demo(port).await;
        }
        Some(Commands::Proxy { listen, target, ui_port, sys }) => {
            start_proxy(listen, target, ui_port, cli.max_history, sys).await;
        }
        Some(Commands::Mock { listen, ui_port }) => {
            start_mock(listen, ui_port, cli.max_history).await;
        }
        None => {
            let saved_config = WireTapConfig::load_or_default(DEFAULT_CONFIG_FILE);
            if let Some(target) = cli.to {
                start_proxy(cli.listen, target, cli.ui_port, cli.max_history, cli.sys).await;
            } else if std::path::Path::new(DEFAULT_CONFIG_FILE).exists() {
                println!("Loaded configuration from {}", DEFAULT_CONFIG_FILE);
                start_proxy(
                    saved_config.listen_port.to_string(),
                    saved_config.target_url,
                    saved_config.ui_port,
                    saved_config.max_history,
                    cli.sys,
                ).await;
            } else {
                println!("No upstream target specified. Starting in demo mode (use --to <URL> to proxy).\n");
                demo::run_demo(cli.ui_port).await;
            }
        }
    }
}

fn parse_port(port_str: &str) -> u16 {
    let clean = port_str.trim().trim_start_matches(':');
    clean.parse::<u16>().unwrap_or(8080)
}

fn normalize_target(target: &str) -> String {
    let t = target.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        t.to_string()
    } else {
        format!("http://{}", t)
    }
}

fn spawn_web_studio(web_state: WebState, ui_port: u16) {
    let web_router = create_web_router(web_state);
    let web_addr = format!("0.0.0.0:{}", ui_port);
    tokio::spawn(async move {
        match tokio::net::TcpListener::bind(&web_addr).await {
            Ok(listener) => {
                let _ = axum::serve(listener, web_router).await;
            }
            Err(e) => {
                eprintln!("{} Failed to bind Web Studio on port {}: {}", "Error:".red().bold(), ui_port, e);
            }
        }
    });
}

async fn start_proxy(listen_str: String, target_str: String, ui_port: u16, max_history: usize, sys: bool) {
    let proxy_port = parse_port(&listen_str);
    let target_base_url = normalize_target(&target_str);
    let store = TrafficStore::new(max_history);

    let mut config = WireTapConfig::load_or_default(DEFAULT_CONFIG_FILE);
    config.listen_port = proxy_port;
    config.target_url = target_base_url.clone();
    config.ui_port = ui_port;
    config.max_history = max_history;
    let _ = config.save_to_file(DEFAULT_CONFIG_FILE);

    let shared_target = Arc::new(RwLock::new(Some(target_base_url.clone())));
    let shared_config = Arc::new(RwLock::new(config));

    let (ca_authority, ca_cert_pem) = build_ca();
    let shared_ca_cert = Arc::new(RwLock::new(ca_cert_pem));

    println!("\nWireTap v{} -- Local API Traffic Inspector & Replay Studio", env!("CARGO_PKG_VERSION"));
    println!("  Proxy listen:      http://127.0.0.1:{}", proxy_port);
    println!("  Forwarding target: {}", target_base_url);
    println!("  Web Studio UI:     http://127.0.0.1:{}", ui_port);

    if sys {
        let mut guard = wiretap::sysproxy::SystemProxyGuard::new();
        if guard.enable(proxy_port) {
            println!("  System proxy:      127.0.0.1:{} (auto-reverts on exit)", proxy_port);
            tokio::spawn(async move {
                let _ = tokio::signal::ctrl_c().await;
                guard.disable();
                println!("\nSystem proxy restored. Exiting.");
                std::process::exit(0);
            });
        }
    }

    // 启动 WebUI 服务
    spawn_web_studio(WebState {
        store: store.clone(),
        target_base_url: shared_target.clone(),
        config: shared_config.clone(),
        config_path: DEFAULT_CONFIG_FILE.to_string(),
        ca_cert_pem: shared_ca_cert,
    }, ui_port);

    println!("  Status:            Running (Press Ctrl+C to stop)\n");
    if let Err(e) = start_hudsucker_proxy(proxy_port, shared_target, store, ca_authority).await {
        eprintln!("Proxy server encountered error: {}", e);
    }
}

async fn start_mock(listen_str: String, ui_port: u16, max_history: usize) {
    let mock_port = parse_port(&listen_str);
    let store = TrafficStore::new(max_history);

    let mut config = WireTapConfig::load_or_default(DEFAULT_CONFIG_FILE);
    config.listen_port = mock_port;
    config.ui_port = ui_port;
    config.max_history = max_history;
    let _ = config.save_to_file(DEFAULT_CONFIG_FILE);

    let shared_target = Arc::new(RwLock::new(None));
    let shared_config = Arc::new(RwLock::new(config));

    println!("\nWireTap v{} -- Mock & Webhook Sink Mode", env!("CARGO_PKG_VERSION"));
    println!("  Mock receiver: http://127.0.0.1:{}", mock_port);
    println!("  Web Studio UI: http://127.0.0.1:{}", ui_port);
    println!("  Status:        Recording incoming requests -> 200 OK (Ctrl+C to stop)\n");

    // 启动 WebUI 服务
    let (_, ca_cert_pem) = build_ca();
    spawn_web_studio(WebState {
        store: store.clone(),
        target_base_url: shared_target,
        config: shared_config,
        config_path: DEFAULT_CONFIG_FILE.to_string(),
        ca_cert_pem: Arc::new(RwLock::new(ca_cert_pem)),
    }, ui_port);

    // 2. Mock 接收 handler
    let mock_store = store.clone();
    let mock_router = axum::Router::new().fallback(move |req: axum::extract::Request| {
        let store = mock_store.clone();
        async move {
            let method = req.method().to_string();
            let uri = req.uri();
            let path = uri.path().to_string();
            let query = uri.query().map(|q| q.to_string());
            let full_url = format!("http://localhost:{}{}", mock_port, uri);

            let mut headers_list = Vec::new();
            for (k, v) in req.headers() {
                headers_list.push(model::KeyValue {
                    key: k.as_str().to_string(),
                    value: v.to_str().unwrap_or("").to_string(),
                });
            }

            let body_bytes = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await.unwrap_or_default();
            let body_str = String::from_utf8(body_bytes.to_vec()).ok();

            let entry = model::TrafficEntry::new(
                method,
                path,
                query,
                full_url,
                headers_list,
                body_str,
                200,
                "OK (Mocked)".to_string(),
                vec![model::KeyValue {
                    key: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                }],
                Some(r#"{"status":"ok","message":"Recorded by WireTap Mock"}"#.to_string()),
                1,
                false,
            );
            store.add(entry).await;

            (axum::http::StatusCode::OK, axum::Json(serde_json::json!({
                "status": "ok",
                "message": "Recorded by WireTap Mock Sink"
            })))
        }
    });

    let mock_addr = format!("0.0.0.0:{}", mock_port);
    let mock_listener = tokio::net::TcpListener::bind(&mock_addr).await.expect("Bind Mock failed");
    let _ = axum::serve(mock_listener, mock_router).await;
}
