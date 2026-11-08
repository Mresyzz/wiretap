use clap::{Parser, Subcommand};
use colored::*;

use wiretap::demo;
use wiretap::model;
use wiretap::proxy::start_hudsucker_proxy;
use wiretap::store::TrafficStore;
use wiretap::web::{create_web_router, WebState};

#[derive(Parser, Debug)]
#[command(
    name = "wiretap",
    author = "WireTap Contributors",
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
        let is_port_syntax = (arg.starts_with(':') && arg.len() > 1 && arg[1..].chars().all(|c| c.is_ascii_digit()))
            || (!arg.starts_with('-') && !is_subcmd && !arg.is_empty() && arg.chars().all(|c| c.is_ascii_digit()));

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
        Some(Commands::Proxy { listen, target, ui_port }) => {
            start_proxy(listen, target, ui_port, cli.max_history).await;
        }
        Some(Commands::Mock { listen, ui_port }) => {
            start_mock(listen, ui_port, cli.max_history).await;
        }
        None => {
            if let Some(target) = cli.to {
                start_proxy(cli.listen, target, cli.ui_port, cli.max_history).await;
            } else {
                println!("{}", "  No upstream target specified (--to <URL>).".yellow());
                println!("  Starting {} with live simulated streams...\n", "DEMO MODE".bold().cyan());
                println!("  {} wiretap --listen 8080 --to http://localhost:3000", "Tip: To proxy real traffic, run:".dimmed());
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

async fn start_proxy(listen_str: String, target_str: String, ui_port: u16, max_history: usize) {
    let proxy_port = parse_port(&listen_str);
    let target_base_url = normalize_target(&target_str);
    let store = TrafficStore::new(max_history);

    println!("{}", "\n========================================================".dimmed());
    println!("{}", "  📡 WIRETAP STUDIO -- ACTIVE PROXY ENGINE".bold().cyan());
    println!("{}", "========================================================".dimmed());
    println!("  {} http://127.0.0.1:{}", "Proxy Listening: ".green().bold(), proxy_port.to_string().yellow().bold());
    println!("  {} {}", "Forwarding Target:".blue().bold(), target_base_url.white().bold());
    println!("  {} http://127.0.0.1:{}", "Web Studio UI:   ".magenta().bold(), ui_port.to_string().yellow().bold());
    println!("  {} Press Ctrl+C anytime to stop\n", "Status:          ".dimmed());

    // 1. 启动 WebUI 服务
    let web_state = WebState {
        store: store.clone(),
        target_base_url: Some(target_base_url.clone()),
    };
    let web_router = create_web_router(web_state);
    let web_addr = format!("0.0.0.0:{}", ui_port);
    let web_listener = match tokio::net::TcpListener::bind(&web_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{} Failed to bind Web Studio on port {}: {}", "Error:".red().bold(), ui_port, e);
            return;
        }
    };
    tokio::spawn(async move {
        let _ = axum::serve(web_listener, web_router).await;
    });

    // 2. 启动基于 Hudsucker 的核心代理服务
    println!("{}", "  🚀 Ready for traffic! Powered by Hudsucker MITM Engine.".green().bold());
    if let Err(e) = start_hudsucker_proxy(proxy_port, Some(target_base_url), store).await {
        eprintln!("Proxy server encountered error: {}", e);
    }
}

async fn start_mock(listen_str: String, ui_port: u16, max_history: usize) {
    let mock_port = parse_port(&listen_str);
    let store = TrafficStore::new(max_history);

    println!("{}", "\n========================================================".dimmed());
    println!("{}", "  📡 WIRETAP STUDIO -- MOCK & SINK MODE".bold().cyan());
    println!("{}", "========================================================".dimmed());
    println!("  {} http://127.0.0.1:{}", "Mock Receiver: ".green().bold(), mock_port.to_string().yellow().bold());
    println!("  {} http://127.0.0.1:{}", "Web Studio UI: ".magenta().bold(), ui_port.to_string().yellow().bold());
    println!("  {} All incoming requests will be recorded and return 200 OK\n", "Info:          ".dimmed());

    // 1. 启动 WebUI 服务
    let web_state = WebState {
        store: store.clone(),
        target_base_url: None,
    };
    let web_router = create_web_router(web_state);
    let web_addr = format!("0.0.0.0:{}", ui_port);
    let web_listener = tokio::net::TcpListener::bind(&web_addr).await.expect("Bind WebUI failed");
    tokio::spawn(async move {
        let _ = axum::serve(web_listener, web_router).await;
    });

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
