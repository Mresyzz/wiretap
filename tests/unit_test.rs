#[cfg(test)]
mod tests {
    use wiretap::model::{KeyValue, TrafficEntry};
    use wiretap::replay::generate_curl;
    use wiretap::store::TrafficStore;

    #[tokio::test]
    async fn test_traffic_store_ring_buffer_eviction() {
        let store = TrafficStore::new(3);

        for i in 1..=5 {
            let entry = TrafficEntry::new(
                "GET".to_string(),
                format!("/api/item/{}", i),
                None,
                format!("http://localhost:3000/api/item/{}", i),
                vec![],
                None,
                200,
                "OK".to_string(),
                vec![],
                None,
                10,
                false,
            );
            store.add(entry).await;
        }

        let list = store.list().await;
        assert_eq!(list.len(), 3);
        // 应该保留最后3条：3, 4, 5
        assert_eq!(list[0].path, "/api/item/3");
        assert_eq!(list[1].path, "/api/item/4");
        assert_eq!(list[2].path, "/api/item/5");
    }

    #[test]
    fn test_generate_curl() {
        let entry = TrafficEntry::new(
            "POST".to_string(),
            "/api/v1/login".to_string(),
            None,
            "http://api.service.internal/api/v1/login".to_string(),
            vec![
                KeyValue {
                    key: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                },
                KeyValue {
                    key: "Authorization".to_string(),
                    value: "Bearer token123".to_string(),
                },
                KeyValue {
                    key: "Host".to_string(),
                    value: "api.service.internal".to_string(),
                }, // Host 应该被过滤掉
            ],
            Some(r#"{"user":"admin"}"#.to_string()),
            200,
            "OK".to_string(),
            vec![],
            None,
            15,
            false,
        );

        let curl = generate_curl(&entry);
        assert!(curl.contains("curl -X POST"));
        assert!(curl.contains("-H \"Content-Type: application/json\""));
        assert!(curl.contains("-H \"Authorization: Bearer token123\""));
        assert!(!curl.contains("-H \"Host:"));
        assert!(curl.contains("-d \"{\\\"user\\\":\\\"admin\\\"}\""));
        assert!(curl.contains("\"http://api.service.internal/api/v1/login\""));
    }

    #[test]
    fn test_rust_embed_static_asset() {
        use wiretap::web::Assets;
        let index = Assets::get("index.html");
        assert!(index.is_some(), "index.html should be embedded in Assets");
        let content = String::from_utf8(index.unwrap().data.to_vec()).expect("valid utf-8");
        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("WIRETAP"));
        assert!(content.contains("/api/ws"));
    }

    #[test]
    fn test_micro_ux_accessibility_attributes() {
        use wiretap::web::{ui, Assets};
        let static_html = String::from_utf8(Assets::get("index.html").unwrap().data.to_vec()).expect("valid utf-8");
        for html in [&static_html, &ui::INDEX_HTML.to_string()] {
            assert!(html.contains(":focus-visible"), "focus-visible styling should be defined");
            assert!(html.contains("aria-label=\"Filter network traffic\""), "filter input aria-label required");
            assert!(html.contains("role=\"status\" aria-live=\"polite\""), "toast aria-live status required");
            assert!(html.contains("tabIndex = 0"), "tabindex required for stream items");
            assert!(html.contains("e.key === \"/\""), "slash keyboard shortcut required");
        }
    }

    #[test]
    fn test_hudsucker_ca_generation() {
        let _ca = wiretap::proxy::build_ca();
        // CA should be generated successfully without panic
    }

    #[tokio::test]
    async fn test_hudsucker_proxy_handler_flow() {
        use hudsucker::hyper::{Request, Response};
        use hudsucker::Body;

        let store = TrafficStore::new(10);
        let mut handler = wiretap::proxy::WireTapProxyHandler::new(
            store.clone(),
            Some("http://127.0.0.1:4000".to_string()),
        );

        // 1. Simulate incoming request
        let req = Request::builder()
            .method("POST")
            .uri("/api/test?debug=1")
            .header("Content-Type", "application/json")
            .header("X-Custom", "wiretap-test")
            .body(Body::from(r#"{"action":"ping"}"#))
            .unwrap();

        let _intercepted = handler.process_request(req).await;

        // 2. Simulate upstream response
        let resp = Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"action":"pong","status":"ok"}"#))
            .unwrap();

        let _returned_resp = handler.process_response(resp).await;

        // 3. Verify TrafficStore received the captured traffic
        let items = store.list().await;
        assert_eq!(items.len(), 1);
        let entry = &items[0];
        assert_eq!(entry.method, "POST");
        assert_eq!(entry.path, "/api/test");
        assert_eq!(entry.query.as_deref(), Some("debug=1"));
        assert_eq!(entry.full_url, "http://127.0.0.1:4000/api/test?debug=1");
        assert_eq!(entry.status, 200);
        assert_eq!(entry.request_body.as_deref(), Some(r#"{"action":"ping"}"#));
        assert_eq!(entry.response_body.as_deref(), Some(r#"{"action":"pong","status":"ok"}"#));
    }

    #[tokio::test]
    async fn test_web_router_endpoints() {
        use tower::ServiceExt;
        use axum::http::{Request, StatusCode};
        use wiretap::web::{create_web_router, WebState};

        let store = TrafficStore::new(10);
        let app = create_web_router(WebState::new(store, None));

        // Test GET /
        let resp = app
            .clone()
            .oneshot(Request::builder().uri("/").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Test GET /api/list
        let resp = app
            .oneshot(Request::builder().uri("/api/list").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_web_socket_realtime_broadcast() {
        use wiretap::web::{create_web_router, WebState};
        use tokio_tungstenite::connect_async;
        use futures_util::StreamExt;

        let store = TrafficStore::new(10);
        let app = create_web_router(WebState::new(store.clone(), None));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let ws_url = format!("ws://127.0.0.1:{}/api/ws", port);
        let (mut ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect to ws");

        // Broadcast an entry into store
        let entry = TrafficEntry::new(
            "GET".to_string(),
            "/ws-ping".to_string(),
            None,
            "http://127.0.0.1/ws-ping".to_string(),
            vec![],
            None,
            200,
            "OK".to_string(),
            vec![],
            Some("pong".to_string()),
            5,
            false,
        );
        store.add(entry).await;

        // Receive from WebSocket
        let msg = ws_stream.next().await.expect("ws message expected").expect("valid msg");
        let text = msg.to_text().expect("text message");
        assert!(text.contains("/ws-ping"));
        assert!(text.contains("pong"));
    }

    #[tokio::test]
    async fn test_adversarial_static_resource_boundary() {
        use tower::ServiceExt;
        use axum::http::{Request, StatusCode};
        use wiretap::web::{create_web_router, WebState};

        let store = TrafficStore::new(10);
        let app = create_web_router(WebState::new(store, None));

        // 1. Existing file: /static/index.html
        let resp = app
            .clone()
            .oneshot(Request::builder().uri("/static/index.html").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let content_type = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(content_type.contains("text/html"));

        // 2. Non-existent file: /static/not_found_test.js
        let resp = app
            .clone()
            .oneshot(Request::builder().uri("/static/not_found_test.js").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        // 3. Path traversal attack attempt: /static/../../Cargo.toml
        let resp = app
            .clone()
            .oneshot(Request::builder().uri("/static/../../Cargo.toml").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        // Axum routing / RustEmbed must not expose files outside static/
        assert!(resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::BAD_REQUEST);

        // 4. Nested non-existent path: /static/css/sub/deep/style.css
        let resp = app
            .oneshot(Request::builder().uri("/static/css/sub/deep/style.css").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_adversarial_proxy_handler_binary_and_malformed() {
        use hudsucker::hyper::{Request as HyperReq, Response as HyperResp};
        use hudsucker::Body;

        let store = TrafficStore::new(10);
        let mut handler = wiretap::proxy::WireTapProxyHandler::new(
            store.clone(),
            Some("http://127.0.0.1:4000/".to_string()), // target with trailing slash
        );

        // 1. Non-UTF8 binary request body (e.g. gzip / protobuf bytes)
        let binary_req_payload: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0x01, 0x80, 0xC0];
        let req = HyperReq::builder()
            .method("PATCH")
            .uri("/custom/path/with%20space?query=val&emoji=%F0%9F%94%A5")
            .header("Content-Type", "application/octet-stream")
            .body(Body::from(binary_req_payload.clone()))
            .unwrap();

        let _ = handler.process_request(req).await;

        // 2. Non-UTF8 binary response body
        let binary_resp_payload: Vec<u8> = vec![0x1F, 0x8B, 0x08, 0x00, 0xDE, 0xAD];
        let resp = HyperResp::builder()
            .status(206)
            .header("Content-Type", "application/octet-stream")
            .body(Body::from(binary_resp_payload))
            .unwrap();

        let _ = handler.process_response(resp).await;

        // Verify TrafficStore state: binary bodies gracefully converted to None without crashing
        let items = store.list().await;
        assert_eq!(items.len(), 1);
        let entry = &items[0];
        assert_eq!(entry.method, "PATCH");
        assert_eq!(entry.status, 206);
        assert_eq!(entry.request_body, None, "Binary non-UTF8 request body should be None");
        assert_eq!(entry.response_body, None, "Binary non-UTF8 response body should be None");
        assert!(entry.full_url.contains("custom/path/with%20space"));
    }

    #[tokio::test]
    async fn test_adversarial_proxy_handler_upstream_error_recording() {
        use hudsucker::hyper::Request as HyperReq;
        use hudsucker::Body;

        let store = TrafficStore::new(10);
        let mut handler = wiretap::proxy::WireTapProxyHandler::new(
            store.clone(),
            Some("http://127.0.0.1:9999".to_string()),
        );

        let req = HyperReq::builder()
            .method("GET")
            .uri("/api/unreachable")
            .body(Body::empty())
            .unwrap();

        let _ = handler.process_request(req).await;

        let resp = hudsucker::hyper::Response::builder()
            .status(502)
            .header("Content-Type", "text/plain")
            .body(Body::from("Bad Gateway (Simulated Connection Error)"))
            .unwrap();
        let _ = handler.process_response(resp).await;

        let items = store.list().await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, 502);
        assert_eq!(items[0].path, "/api/unreachable");
        assert_eq!(items[0].response_body.as_deref(), Some("Bad Gateway (Simulated Connection Error)"));
    }

    #[tokio::test]
    async fn test_adversarial_websocket_concurrent_clients() {
        use wiretap::web::{create_web_router, WebState};
        use tokio_tungstenite::connect_async;
        use futures_util::StreamExt;

        let store = TrafficStore::new(50);
        let app = create_web_router(WebState::new(store.clone(), None));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let ws_url = format!("ws://127.0.0.1:{}/api/ws", port);

        // Connect 3 concurrent WebSocket clients
        let (mut ws1, _) = connect_async(&ws_url).await.expect("Client 1 connect");
        let (mut ws2, _) = connect_async(&ws_url).await.expect("Client 2 connect");
        let (mut ws3, _) = connect_async(&ws_url).await.expect("Client 3 connect");

        // Broadcast a traffic item
        let entry1 = TrafficEntry::new(
            "POST".to_string(),
            "/event/1".to_string(),
            None,
            "http://127.0.0.1/event/1".to_string(),
            vec![],
            None,
            200,
            "OK".to_string(),
            vec![],
            Some("payload-1".to_string()),
            10,
            false,
        );
        store.add(entry1).await;

        // All 3 clients should receive the event
        for (i, ws) in [&mut ws1, &mut ws2, &mut ws3].iter_mut().enumerate() {
            let msg = ws.next().await.expect("message expected").expect("valid msg");
            let text = msg.to_text().expect("text");
            assert!(text.contains("/event/1"), "Client {} must receive entry 1", i + 1);
        }

        // Abruptly close Client 1
        let _ = ws1.close(None).await;
        drop(ws1);

        // Broadcast a second traffic item
        let entry2 = TrafficEntry::new(
            "GET".to_string(),
            "/event/2".to_string(),
            None,
            "http://127.0.0.1/event/2".to_string(),
            vec![],
            None,
            200,
            "OK".to_string(),
            vec![],
            Some("payload-2".to_string()),
            15,
            false,
        );
        store.add(entry2).await;

        // Client 2 and 3 must continue receiving without disruption
        for (i, ws) in [&mut ws2, &mut ws3].iter_mut().enumerate() {
            let msg = ws.next().await.expect("message expected").expect("valid msg");
            let text = msg.to_text().expect("text");
            assert!(text.contains("/event/2"), "Remaining client {} must receive entry 2", i + 2);
        }
    }

    #[tokio::test]
    async fn test_adversarial_traffic_store_concurrency_stress() {
        use std::sync::Arc;

        let store = TrafficStore::new(20); // Small ring buffer capacity
        let store_arc = Arc::new(store);

        let mut handles = Vec::new();
        for worker_id in 0..10 {
            let s = store_arc.clone();
            let handle = tokio::spawn(async move {
                for i in 0..15 {
                    let entry = TrafficEntry::new(
                        "GET".to_string(),
                        format!("/worker/{}/item/{}", worker_id, i),
                        None,
                        format!("http://127.0.0.1/worker/{}/item/{}", worker_id, i),
                        vec![],
                        None,
                        200,
                        "OK".to_string(),
                        vec![],
                        None,
                        1,
                        false,
                    );
                    s.add(entry).await;
                }
            });
            handles.push(handle);
        }

        for h in handles {
            h.await.unwrap();
        }

        let list = store_arc.list().await;
        assert_eq!(list.len(), 20, "Store should retain exactly max_capacity entries under concurrent writes");

        store_arc.clear().await;
        assert_eq!(store_arc.list().await.len(), 0, "Store should be empty after clear");
    }

    #[test]
    fn test_adversarial_curl_special_characters() {
        let entry = TrafficEntry::new(
            "POST".to_string(),
            "/api/v1/data".to_string(),
            Some("q=1&filter=\"test\"".to_string()),
            "http://localhost:8080/api/v1/data?q=1&filter=\"test\"".to_string(),
            vec![
                KeyValue {
                    key: "Authorization".to_string(),
                    value: "Bearer \"quoted-token\"".to_string(),
                },
                KeyValue {
                    key: "host".to_string(), // Lowercase host should be filtered
                    value: "localhost:8080".to_string(),
                },
                KeyValue {
                    key: "CONTENT-LENGTH".to_string(), // Uppercase content-length should be filtered
                    value: "100".to_string(),
                },
            ],
            Some("{\"message\": \"line1\\nline2\", \"status\": \"ok\"}".to_string()),
            200,
            "OK".to_string(),
            vec![],
            None,
            5,
            false,
        );

        let curl = generate_curl(&entry);
        assert!(!curl.contains("-H \"host:"), "Host header should be excluded");
        assert!(!curl.contains("-H \"CONTENT-LENGTH:"), "Content-Length header should be excluded");
        assert!(curl.contains("-H \"Authorization: Bearer \\\"quoted-token\\\"\""), "Quotes in headers should be escaped");
        assert!(curl.contains("-d \"{\\\"message\\\": \\\"line1\\nline2\\\", \\\"status\\\": \\\"ok\\\"}\""), "Body quotes should be escaped");
    }

    #[tokio::test]
    async fn test_adversarial_websocket_burst_lag_behavior() {
        use wiretap::web::{create_web_router, WebState};
        use tokio_tungstenite::connect_async;
        use futures_util::StreamExt;

        let store = TrafficStore::new(1000);
        let app = create_web_router(WebState::new(store.clone(), None));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let ws_url = format!("ws://127.0.0.1:{}/api/ws", port);
        let (mut ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect to ws");

        // Broadcast 600 entries rapidly into store WITHOUT reading from ws_stream yet
        for i in 0..600 {
            let entry = TrafficEntry::new(
                "GET".to_string(),
                format!("/flood/{}", i),
                None,
                format!("http://127.0.0.1/flood/{}", i),
                vec![],
                None,
                200,
                "OK".to_string(),
                vec![],
                None,
                1,
                false,
            );
            store.add(entry).await;
        }

        let mut received = 0;
        loop {
            match tokio::time::timeout(std::time::Duration::from_millis(200), ws_stream.next()).await {
                Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Text(_)))) => {
                    received += 1;
                }
                _ => break,
            }
        }
        println!("WebSocket client received {} messages before stream idle/ended", received);
        assert!(received > 0, "Client must receive messages");
    }

    #[tokio::test]
    async fn test_adversarial_web_api_error_paths() {
        use tower::ServiceExt;
        use axum::http::{Request, StatusCode};
        use wiretap::web::{create_web_router, WebState};

        let store = TrafficStore::new(10);
        let app = create_web_router(WebState::new(store, None));

        // 1. GET /api/replay/nonexistent -> 404
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/replay/nonexistent-id-12345")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        // 2. GET /api/curl/nonexistent -> 404
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/curl/nonexistent-id-12345")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        // 3. POST /api/clear -> 200 OK
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/clear")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 4. GET /api/list -> 200 OK
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/list")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_config_api_and_persistence() {
        use tower::ServiceExt;
        use axum::http::{Request, StatusCode};
        use wiretap::web::{create_web_router, WebState};
        use wiretap::config::WireTapConfig;
        use http_body_util::BodyExt;

        let temp_dir = std::env::temp_dir();
        let test_config_path = temp_dir.join(format!("wiretap_test_cfg_{}.json", uuid::Uuid::new_v4()));
        
        let store = TrafficStore::new(10);
        let target_holder = std::sync::Arc::new(tokio::sync::RwLock::new(Some("http://initial-host:3000".to_string())));
        let cfg = WireTapConfig {
            listen_port: 8080,
            target_url: "http://initial-host:3000".to_string(),
            ui_port: 9090,
            max_history: 1000,
        };
        let cfg_holder = std::sync::Arc::new(tokio::sync::RwLock::new(cfg));

        let state = WebState {
            store,
            target_base_url: target_holder.clone(),
            config: cfg_holder.clone(),
            config_path: test_config_path.to_string_lossy().to_string(),
        };
        let app = create_web_router(state);

        // 1. GET /api/config
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let read_cfg: WireTapConfig = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(read_cfg.listen_port, 8080);
        assert_eq!(read_cfg.target_url, "http://initial-host:3000");

        // 2. POST /api/config - Update configuration dynamically
        let new_cfg = WireTapConfig {
            listen_port: 8888,
            target_url: "http://updated-upstream:5000".to_string(),
            ui_port: 9091,
            max_history: 2000,
        };
        let post_body = serde_json::to_string(&new_cfg).unwrap();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(post_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify in-memory dynamic hot-update took effect
        let current_target = target_holder.read().await.clone();
        assert_eq!(current_target.as_deref(), Some("http://updated-upstream:5000"));

        // Verify config was persisted to file on disk
        assert!(test_config_path.exists());
        let disk_cfg = WireTapConfig::load_or_default(&test_config_path);
        assert_eq!(disk_cfg.listen_port, 8888);
        assert_eq!(disk_cfg.target_url, "http://updated-upstream:5000");
        assert_eq!(disk_cfg.ui_port, 9091);
        assert_eq!(disk_cfg.max_history, 2000);

        // Clean up temp file
        let _ = std::fs::remove_file(test_config_path);
    }
}

