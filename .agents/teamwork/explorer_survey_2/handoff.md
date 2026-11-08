# Handoff Report: Core Proxy & Realtime Backend Survey

## 1. Observation (客观观察)

### 1.1 依赖定义 (`Cargo.toml`)
- **文件路径**: `e:\Code\githubidk\Cargo.toml`
- **依赖声明 (Lines 9-23)**:
  ```toml
  [dependencies]
  tokio = { version = "1.38", features = ["full"] }
  axum = { version = "0.7", features = ["macros"] }
  tower = { version = "0.4" }
  tower-http = { version = "0.5", features = ["cors"] }
  reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "json"] }
  clap = { version = "4.5", features = ["derive"] }
  serde = { version = "1.0", features = ["derive"] }
  serde_json = "1.0"
  chrono = { version = "0.4", features = ["serde"] }
  uuid = { version = "1.8", features = ["v4", "serde"] }
  colored = "2.1"
  tokio-stream = { version = "0.1", features = ["sync"] }
  futures-util = "0.3"
  ```
- **关键缺失事实**:
  - `Cargo.toml` 与 `Cargo.lock` 中均**不存在** `hudsucker` 依赖。
  - `Cargo.toml` 与 `Cargo.lock` 中均**不存在** `rust-embed` 依赖。
  - `axum` 的特性配置仅为 `features = ["macros"]`，**未开启** `ws` 特性。

### 1.2 核心代理实现 (`src/proxy.rs` & `src/main.rs`)
- **文件路径**: `e:\Code\githubidk\src\main.rs` (Lines 147-167)
  ```rust
  let proxy_config = Arc::new(ProxyConfig {
      target_base_url,
      store,
  });
  let proxy_router = axum::Router::new()
      .fallback(proxy_handler)
      .with_state(proxy_config);

  let proxy_addr = format!("0.0.0.0:{}", proxy_port);
  let proxy_listener = match tokio::net::TcpListener::bind(&proxy_addr).await { ... };
  axum::serve(proxy_listener, proxy_router).await
  ```
- **文件路径**: `e:\Code\githubidk\src\proxy.rs` (Lines 18-21, 72-90, 124-144)
  - `proxy_handler` 作为 Axum fallback 处理器：
    ```rust
    pub async fn proxy_handler(
        State(config): State<Arc<ProxyConfig>>,
        req: Request,
    ) -> Response
    ```
  - 解析目标 URL 并使用 `reqwest::Client` 转发：
    ```rust
    let client = reqwest::Client::builder().build().unwrap_or_else(|_| reqwest::Client::new());
    let mut req_builder = client.request(reqwest_method, &target_url).headers(client_headers);
    if !body_bytes.is_empty() { req_builder = req_builder.body(body_bytes.clone()); }
    let upstream_result = req_builder.send().await;
    ```
  - 将捕获的 HTTP 记录存入 `store.add(entry).await`，并将 `reqwest` 响应转换为 Axum `Response` 返回。
- **关于 `tokio::net::TcpStream`**:
  - 全局代码检索 `grep_search "TcpStream"` 结果为 0。
  - 代码仅使用 `tokio::net::TcpListener::bind` 为 Axum 提供监听套接字，**完全未裸写** `TcpStream` 代理协议。

### 1.3 Web 服务器与静态资源服务 (`src/web/mod.rs` & `src/web/ui.rs`)
- **文件路径**: `e:\Code\githubidk\src\web\mod.rs` (Lines 26-39)
  ```rust
  pub fn create_web_router(state: WebState) -> Router {
      Router::new()
          .route("/", get(index_handler))
          .route("/api/list", get(list_handler))
          .route("/api/events", get(sse_handler))
          .route("/api/replay/:id", post(replay_handler))
          .route("/api/curl/:id", get(curl_handler))
          .route("/api/clear", post(clear_handler))
          .with_state(Arc::new(state))
  }

  async fn index_handler() -> Html<&'static str> {
      Html(ui::INDEX_HTML)
  }
  ```
- **文件路径**: `e:\Code\githubidk\src\web\ui.rs` (Line 1)
  ```rust
  pub const INDEX_HTML: &str = r#"<!DOCTYPE html>...
  ```
- **静态资源提供方式**:
  - 静态页面直接以单个静态字符串常量 `pub const INDEX_HTML: &str` 硬编码在 Rust 源码中，通过 `Html(ui::INDEX_HTML)` 返回。
  - **未使用** `rust-embed` crate，也没有独立的资源目录。

### 1.4 数据包广播与实时推送机制 (`src/store.rs` & `src/web/mod.rs` & `src/web/ui.rs`)
- **文件路径**: `e:\Code\githubidk\src\store.rs` (Lines 8-16, 32-33, 50-52)
  ```rust
  pub struct TrafficStore {
      max_capacity: usize,
      entries: Arc<RwLock<VecDeque<TrafficEntry>>>,
      sender: broadcast::Sender<TrafficEntry>,
  }

  impl TrafficStore {
      pub fn new(max_capacity: usize) -> Self {
          let (sender, _) = broadcast::channel(500);
          ...
      }
      pub async fn add(&self, entry: TrafficEntry) {
          ...
          let _ = self.sender.send(entry);
      }
      pub fn subscribe(&self) -> broadcast::Receiver<TrafficEntry> {
          self.sender.subscribe()
      }
  }
  ```
  - `TrafficStore` 内部**确实使用**了 `tokio::sync::broadcast::channel(500)`。
- **文件路径**: `e:\Code\githubidk\src\web\mod.rs` (Lines 46-62)
  - 实时推送使用的是 Axum SSE (Server-Sent Events)，并非 WebSocket：
  ```rust
  async fn sse_handler(
      State(state): State<Arc<WebState>>,
  ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
      let rx = state.store.subscribe();
      let stream = BroadcastStream::new(rx).filter_map(|res| match res {
          Ok(entry) => {
              if let Ok(json) = serde_json::to_string(&entry) {
                  Some(Ok(Event::default().data(json)))
              } else {
                  None
              }
          }
          Err(_) => None,
      });

      Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
  }
  ```
- **文件路径**: `e:\Code\githubidk\src\web\ui.rs` (Lines 730-751)
  - 前端 JavaScript 使用 `const eventSource = new EventSource("/api/events");` 接收实时流量。

---

## 2. Logic Chain (推理链条)

### 2.1 核心代理实现推理
1. 观察到 `Cargo.toml` 中无 `hudsucker`，全局代码检索无 `TcpStream`。
2. 观察到 `src/proxy.rs` 使用 `axum::Router::fallback(proxy_handler)` 与 `reqwest::Client`。
3. **推理**:
   - 现存代理实现是**反向 HTTP 代理 (Reverse Proxy)**，必须绑定特定上游 `--to <URL>` 才能转发。
   - 它**杜绝了**使用裸 `TcpStream` 带来的内存所有权与生命周期缺陷（满足了 R3 中“严禁裸写 TcpStream”这一负向约束）。
   - 但是它**未集成** `hudsucker`，无法作为正向 MITM 代理（如浏览器配置 `HTTP_PROXY` 时截获 HTTPS CONNECT 隧道并进行中间人解密/重加密），违反了 R3 中“核心代理：基于 `hudsucker`”的正向约束。

### 2.2 Web 服务器与静态资源服务推理
1. 观察到 `Cargo.toml` 包含 `axum = { version = "0.7" }`，`src/web/mod.rs` 使用 `Router` 与 `axum::serve`。
2. 观察到 `src/web/mod.rs` 的 `index_handler` 直接返回 `Html(ui::INDEX_HTML)`，而 `ui::INDEX_HTML` 是位于 `src/web/ui.rs` 中的 771 行 Rust raw string 常量。
3. **推理**:
   - Web 服务器确实基于 `axum` 构建。
   - 但是单二进制打包当前是通过 Rust 语言内置的字符串字面量嵌入，**没有使用** `rust-embed`。这违反了 R3 架构约束中的“Web 服务基于 `axum` + `rust-embed` 实现静态资源内嵌与单二进制打包”。

### 2.3 实时数据包推送推理
1. 观察到 `src/store.rs` 创建了 `broadcast::channel(500)` 并对外暴露 `subscribe()`。
2. 观察到 `src/web/mod.rs` 暴露 `/api/events`，通过 `BroadcastStream` 与 Axum `Sse` 推送。
3. 观察到前端使用 `EventSource("/api/events")` 消费。
4. **推理**:
   - 后端事件总线已经正确采用了 `tokio::sync::broadcast::channel`。
   - 但是通信协议层使用了 HTTP SSE（Server-Sent Events），而非 Axum 原生 WebSocket。
   - 这违反了 R3 架构约束中的“实时推送：使用 Axum 内置的 WebSocket，捕获数据包后通过 `tokio::sync::broadcast::channel` 向前端实时广播 JSON 数据”。

---

## 3. Caveats (局限与未涉领域)

1. **hudsucker 运行时环境与依赖体积**: `hudsucker` 会引入 `rustls` / `openssl` 以及证书颁发机构（如 `rcgen`）的开销，在 Windows / Linux / macOS 下单二进制构建体积需要结合 `Cargo.toml` 的 profile release 优化进行验证。
2. **渐进式迁移考量**: 现有 `wiretap demo` 和反向代理模式（`--to`）目前已能独立运行并通过现有单元测试。引入 `hudsucker` 和 Axum WebSocket 时，应注意保持或兼容当前的反向代理与 mock 工作流，不破坏现有 CLI 使用体验与测试用例。
3. **前端 UI 与 Palette 的协同**: 将 SSE 改为 WebSocket（如 `/api/ws`）需要同步微调前端 `src/web/ui.rs` 的连接代码，需与负责 UX/UI 的 Palette 智能体保持接口协议一致。

---

## 4. Conclusion (分析结论与架构整改建议)

### 4.1 现有实现合规度对照矩阵

| 架构需求项 | 规范要求 | 现状实现 | 合规状态 | 影响与改动点 |
| :--- | :--- | :--- | :---: | :--- |
| **代理协议底层** | 严禁使用 `tokio::net::TcpStream` 裸写 | 使用 `axum` + `reqwest::Client` | **合规 (Pass)** | 无裸写 TcpStream 缺陷 |
| **核心代理库** | 基于 `hudsucker` MITM 代理 | 自定义 Axum 反向代理 | **违背 (Violation)** | 需在 `Cargo.toml` 引入 `hudsucker`，基于 `HttpHandler` 回调实现请求/响应捕获 |
| **Web 框架** | 基于 `axum` | `axum = 0.7` | **合规 (Pass)** | 符合规范 |
| **静态资源嵌入** | 基于 `rust-embed` 打包嵌入二进制 | `ui::INDEX_HTML` 纯字符串字面量 | **违背 (Violation)** | 需引入 `rust-embed`，规范化静态文件打包与路由分发 |
| **内部事件总线** | `tokio::sync::broadcast::channel` | `broadcast::channel(500)` | **合规 (Pass)** | `TrafficStore` 已完全具备 |
| **实时推送协议** | Axum 内置 WebSocket | Axum SSE (`/api/events`) | **违背 (Violation)** | 需开启 `axum` 的 `ws` feature，新增 `/api/ws` 路由并替换前端 `EventSource` |

### 4.2 队长 (Leader) 实施整改方案建议

#### 改动 1: `Cargo.toml` 依赖补充
```toml
[dependencies]
# 开启 axum 的 ws 特性
axum = { version = "0.7", features = ["macros", "ws"] }
# 引入 rust-embed 用于静态资源打包
rust-embed = "8.5"
# 引入 hudsucker 用于正向/MITM 代理拦截
hudsucker = { version = "0.20", features = ["rustls-client"] }
rcgen = "0.13" # 用于 MITM 证书动态签发
```

#### 改动 2: 静态资源内嵌 (`rust-embed`)
- 创建静态资源目录（例如 `static/index.html`，或通过 `RustEmbed` 结构体绑定）：
```rust
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static/"] // 或内嵌前端资源文件
pub struct Asset;
```
- 在 `src/web/mod.rs` 中提供静态资源服务，替代纯字符串常量渲染。

#### 改动 3: 实时推送升级至 Axum WebSocket
- 在 `src/web/mod.rs` 中新增 `/api/ws` 路由：
```rust
use axum::extract::ws::{WebSocket, WebSocketUpgrade, Message};

pub fn create_web_router(state: WebState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/list", get(list_handler))
        .route("/api/ws", get(ws_handler)) // WebSocket 实时推送
        .route("/api/events", get(sse_handler)) // 可保留做向下兼容
        ...
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<WebState>) {
    let mut rx = state.store.subscribe();
    while let Ok(entry) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&entry) {
            if socket.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    }
}
```
- 在前端 `src/web/ui.rs` 中由 Palette 协同将 `new EventSource("/api/events")` 替换为 `new WebSocket(...)`。

#### 改动 4: 核心代理集成 `hudsucker`
- 新增 `hudsucker` 适配层（如 `src/proxy/mitm.rs` 或重写 `src/proxy.rs`）：
- 实现 `hudsucker::HttpHandler` 接口，在 `handle_request` 和 `handle_response` 中提取方法、URL、Headers、Body 与耗时，构造 `TrafficEntry` 并推入 `TrafficStore`。

---

## 5. Verification Method (独立复现与验证方式)

1. **依赖与编译检查**:
   - 执行 `cargo check` 确认无编译错误与警告。
   - 执行 `cargo test` 确认单元测试 (`tests/unit_test.rs`) 全部通过：
     ```powershell
     cargo test
     ```
2. **代码定位核实**:
   - 查看 `src/proxy.rs:18-89` 确认当前代理实现机制。
   - 查看 `src/web/mod.rs:30,46-62` 确认当前推送仅有 SSE 无 WebSocket。
   - 查看 `src/web/mod.rs:37-39` 与 `src/web/ui.rs:1` 确认当前使用纯字符串字面量而非 `rust-embed`。
   - 查看 `src/store.rs:16,32,50` 确认广播通道存在。
3. **失效条件 (Invalidation Conditions)**:
   - 若上游指令或原始需求变更允许保留 SSE 而非强制 WebSocket，则推送部分违规项自动失效。
   - 若单二进制打包仅要求“嵌入单文件”而接受 `&'static str` 常量，则无需引入 `rust-embed`。但在 `ORIGINAL_REQUEST.md` 明确指定 `axum + rust-embed` 的前提下，该违规项明确成立。
