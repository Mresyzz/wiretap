use std::time::Duration;
use colored::*;
use tokio::time::sleep;

use crate::model::{KeyValue, TrafficEntry};
use crate::store::TrafficStore;
use crate::web::{create_web_router, WebState};

pub async fn run_demo(web_port: u16) {
    let store = TrafficStore::new(1000);

    println!("\nWireTap v{} -- Zero-Setup Demo Mode", env!("CARGO_PKG_VERSION"));
    println!("  Web Studio UI: http://127.0.0.1:{}", web_port);
    println!("  Traffic source: Simulated live microservice streams");
    println!("  Status:         Active (Press Ctrl+C to exit)\n");

    // 预置初始真实示例流量
    seed_initial_traffic(&store).await;

    // 启动嵌入式 Web 服务
    let cfg = crate::config::WireTapConfig {
        listen_port: 8080,
        target_url: "http://demo.internal.api".to_string(),
        ui_port: web_port,
        max_history: 1000,
        target_process: None,
    };
    let (_, ca_pem) = crate::proxy::build_ca();
    let web_state = WebState {
        store: store.clone(),
        target_base_url: std::sync::Arc::new(tokio::sync::RwLock::new(Some("http://demo.internal.api".to_string()))),
        target_process: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
        config: std::sync::Arc::new(tokio::sync::RwLock::new(cfg)),
        config_path: crate::config::DEFAULT_CONFIG_FILE.to_string(),
        ca_cert_pem: std::sync::Arc::new(tokio::sync::RwLock::new(ca_pem)),
    };
    let app = create_web_router(web_state);
    let addr = format!("0.0.0.0:{}", web_port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{} Failed to bind web studio on port {}: {}", "Error:".red().bold(), web_port, e);
            return;
        }
    };

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    println!("{}", "  Studio is listening. Open your browser to view incoming streams.".green());

    // 持续生成动态流量
    let mock_endpoints = vec![
        ("GET", "/api/v1/users/profile", 200, "OK", r#"{"id":"usr_8921","name":"Alex Chen","role":"admin","teams":["core-infra","ai-sec"]}"#, 8, "chrome.exe", 10248),
        ("POST", "/api/v1/orders/create", 201, "Created", r#"{"order_id":"ord_55214","status":"pending_payment","amount":189.50,"currency":"USD"}"#, 42, "node.exe", 8452),
        ("GET", "/api/v1/metrics/prometheus", 200, "OK", "cpu_usage_ratio{zone=\"us-east\"} 0.28\nmemory_allocated_bytes 419430400", 3, "python.exe", 14320),
        ("GET", "/api/v1/discounts/coupon-xyz", 404, "Not Found", r#"{"error":"Coupon expired or invalid","code":"ERR_COUPON_NOT_FOUND"}"#, 12, "chrome.exe", 10248),
        ("POST", "/api/v1/payments/stripe-intent", 200, "OK", r#"{"client_secret":"pi_3MtwBwLkdIwHu7ix28a3tqPa_secret_yr89","status":"requires_action"}"#, 65, "curl.exe", 6210),
        ("DELETE", "/api/v1/cache/redis/keys", 204, "No Content", "", 5, "node.exe", 8452),
        ("POST", "/api/v1/ai/completions", 500, "Internal Server Error", r#"{"error":"Upstream LLM provider timeout after 30000ms","status":500}"#, 850, "python.exe", 14320),
    ];

    let mut idx = 0;
    loop {
        sleep(Duration::from_millis(2200)).await;

        let (method, path, status, status_text, body, duration, proc_name, proc_pid) = mock_endpoints[idx % mock_endpoints.len()];
        idx += 1;

        let entry = TrafficEntry::new(
            method.to_string(),
            path.to_string(),
            None,
            format!("http://demo.internal.api{}", path),
            vec![
                KeyValue { key: "Host".to_string(), value: "demo.internal.api".to_string() },
                KeyValue { key: "User-Agent".to_string(), value: "WireTap-Agent/1.0".to_string() },
                KeyValue { key: "Accept".to_string(), value: "application/json".to_string() },
            ],
            if method == "POST" { Some(r#"{"action":"sync","timestamp":1731000000}"#.to_string()) } else { None },
            status,
            status_text.to_string(),
            vec![
                KeyValue { key: "Content-Type".to_string(), value: "application/json; charset=utf-8".to_string() },
                KeyValue { key: "Server".to_string(), value: "nginx/1.25".to_string() },
            ],
            if body.is_empty() { None } else { Some(body.to_string()) },
            duration,
            false,
        ).with_process(Some(proc_name.to_string()), Some(proc_pid));

        store.add(entry).await;
        let status_colored = if status >= 200 && status < 300 {
            status.to_string().green()
        } else if status >= 400 && status < 500 {
            status.to_string().yellow()
        } else {
            status.to_string().red()
        };
        println!("  [{}] {:<28} -> {} ({}ms)", method, path, status_colored, duration);
    }
}

async fn seed_initial_traffic(store: &TrafficStore) {
    let seeds = vec![
        TrafficEntry::new(
            "POST".to_string(),
            "/api/v1/auth/login".to_string(),
            None,
            "http://demo.internal.api/api/v1/auth/login".to_string(),
            vec![
                KeyValue { key: "Content-Type".to_string(), value: "application/json".to_string() },
                KeyValue { key: "User-Agent".to_string(), value: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string() },
            ],
            Some(r#"{"username":"developer@company.com","auth_type":"webauthn"}"#.to_string()),
            200,
            "OK".to_string(),
            vec![
                KeyValue { key: "Content-Type".to_string(), value: "application/json".to_string() },
                KeyValue { key: "Set-Cookie".to_string(), value: "session=wt_sess_token_901; HttpOnly".to_string() },
            ],
            Some(r#"{"token":"jwt.eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...","expires_in":86400,"user":{"id":"usr_01","email":"developer@company.com"}}"#.to_string()),
            38,
            false,
        ).with_process(Some("chrome.exe".to_string()), Some(10248)),
        TrafficEntry::new(
            "GET".to_string(),
            "/api/v1/dashboard/overview".to_string(),
            Some("range=7d".to_string()),
            "http://demo.internal.api/api/v1/dashboard/overview?range=7d".to_string(),
            vec![
                KeyValue { key: "Authorization".to_string(), value: "Bearer jwt.eyJhbGciOi...".to_string() },
            ],
            None,
            200,
            "OK".to_string(),
            vec![
                KeyValue { key: "Content-Type".to_string(), value: "application/json".to_string() },
                KeyValue { key: "X-Cache".to_string(), value: "HIT".to_string() },
            ],
            Some(r#"{"daily_active_users":14250,"total_revenue":89200.00,"server_health":"healthy"}"#.to_string()),
            14,
            false,
        ).with_process(Some("node.exe".to_string()), Some(8452)),
    ];

    for s in seeds {
        store.add(s).await;
    }
}
