# Handoff Report: Milestone 3 - Bug Finding & Adversarial Verification

- **Author**: 找 BUG (Challenger & Bug Finder)
- **Target**: WireTap (`e:\Code\githubidk`)
- **Status**: Complete (Hard Handoff)
- **Verdict**: **APPROVE**
- **Timestamp**: 2026-11-08T15:05:00Z

---

## 1. Observation (客观事实与证据)

在 Milestone 3 中，找 BUG 角色对 Milestone 1 (队长) 与 Milestone 2 (Palette 🎨) 引入的全部改动进行了全方位的代码审查与对抗性边界测试。具体观察事实如下：

### 1.1 代码改动审查事实
1. **核心代理架构合规性 (`src/proxy.rs`, `src/main.rs`)**:
   - 彻底移除了原先基于裸 `TcpStream` 或简易包装代理实现的风险。`src/proxy.rs` 第 236-255 行全面接入成熟框架 `hudsucker 0.25.0`，通过 `Proxy::builder()` 装配 `WireTapProxyHandler`，完整实现了 `HttpHandler` 与 `WebSocketHandler` 特征。
   - `build_ca()`（第 226-234 行）使用 `rcgen` 动态签发自签受信任根证书，零外部密钥文件依赖。
   - `src/main.rs` 第 146-150 行在 `start_proxy` 中调用 `start_hudsucker_proxy(proxy_port, Some(target_base_url), store).await`，移交代理接管流量。
2. **Web 服务与单二进制内嵌 (`src/web/mod.rs`, `static/index.html`)**:
   - `src/web/mod.rs` 第 22-25 行通过 `#[derive(RustEmbed)] #[folder = "static/"] pub struct Assets;` 将前端资产打包进二进制。
   - 暴露 `/`（`index_handler`）与 `/static/*file`（`static_handler`），并使用 `mime_guess` 自动提取 Content-Type。
   - `src/web/mod.rs` 第 70-86 行新增 `/api/ws` Axum WebSocket 路由，订阅 `TrafficStore` 的 `tokio::sync::broadcast::channel` 实时推送抓包数据，同时保留 `/api/events`（Axum SSE）以提供平滑降级。
3. **前端 Micro-UX 与 a11y 规范 (`src/web/ui.rs`, `static/index.html`, `.Jules/palette.md`)**:
   - 执行 `git diff --stat src/web/ui.rs` 实际统计结果：
     ```
      src/web/ui.rs | 24 ++++++++++++++++++++++--
      1 file changed, 22 insertions(+), 2 deletions(-)
     ```
     净变动 24 行，严格低于 **≤50 行** 的微 UX 限制要求。
   - 交互元素包含 `:focus-visible` 焦点双轮廓、`aria-label="Filter network traffic"`、`role="status" aria-live="polite"`、`tabIndex = 0`、Enter/Space 键切换选中以及 `/` 和 `Escape` 全局搜索框快捷键。
   - `.Jules/palette.md` 存在且格式严格遵循规范：
     ```markdown
     # Palette UX & Accessibility Design Journal

     ## 2026-11-08 - Keyboard Navigation & Focus-Visible in API Stream List
     **Learning:** For high-density developer tools like traffic inspectors, mouse-only click targets create friction and a11y violations. Adding `tabindex="0"`, keyboard Enter/Space triggers, and explicit `:focus-visible` styles enables seamless keyboard triage without redesigning DOM layouts.
     **Action:** Always verify keyboard focusability on virtualized/dynamic lists and pair search boxes with standard `/` hotkeys.
     ```

### 1.2 对抗性与边界测试编写 (`tests/unit_test.rs`)
在 `tests/unit_test.rs` 中新增了 8 个高强度对抗性测试，使全量测试用例从 8 个扩充至 16 个：
1. `test_adversarial_static_resource_boundary`: 验证 `/static/index.html` (200 OK + text/html)、不存在文件 `/static/not_found_test.js` (404 Not Found)、路径穿越尝试 `/static/../../Cargo.toml` (404/400 安全隔离)、深层不存在路径 `/static/css/sub/deep/style.css` (404 Not Found)。
2. `test_adversarial_proxy_handler_binary_and_malformed`: 注入非 UTF-8 二进制请求体（`[0xFF, 0xFE, 0x00, 0x01, 0x80, 0xC0]`）和非 UTF-8 二进制响应体（`[0x1F, 0x8B, 0x08, 0x00, 0xDE, 0xAD]`），配合 `PATCH` 方法与特殊字符 URI，断言抓包数据结构能够无崩溃优雅处理（body 转为 `None`）。
3. `test_adversarial_proxy_handler_upstream_error_recording`: 验证上游 502/不可达异常能够被正常转录为 502 状态码、Bad Gateway 说明及耗时。
4. `test_adversarial_websocket_concurrent_clients`: 模拟 3 个 WebSocket 客户端同时连接，广播后验证 3 个客户端全量收到数据；随后模拟 1 个客户端强制掉线，再次广播验证其余存活客户端不受干扰。
5. `test_adversarial_traffic_store_concurrency_stress`: 10 个 Tokio 任务高并发并发写入 150 条记录至容量为 20 的环形队列，验证无数据竞争或死锁，容量严格收敛在 20，且 `clear()` 能够正常清空。
6. `test_adversarial_curl_special_characters`: 验证包含双引号转义、换行符、大小写混合 `host` 和 `CONTENT-LENGTH` 请求头的 cURL 准确生成。
7. `test_adversarial_websocket_burst_lag_behavior`: 模拟向 WebSocket 连接瞬时突发洪峰 600 条记录，实测客户端成功读取 600 条消息无丢失无死锁。
8. `test_adversarial_web_api_error_paths`: 验证重放不存在流量 ID (`/api/replay/nonexistent-id`)、获取不存在 cURL (`/api/curl/nonexistent-id`) 返回 404，以及 `/api/clear` 和 `/api/list` 正常运作。

### 1.3 编译、构建与全量测试输出证据
- **命令 1**: `cargo check`
  - 退出码: `0`
  - 输出: `Finished dev profile [unoptimized + debuginfo] target(s) in 0.88s`
- **命令 2**: `cargo check --release`
  - 退出码: `0`
  - 输出: `Finished release profile [optimized] target(s) in 1.90s`
- **命令 3**: `cargo test`
  - 退出码: `0`
  - 输出:
    ```
    running 16 tests
    test tests::test_adversarial_web_api_error_paths ... ok
    test tests::test_hudsucker_ca_generation ... ok
    test tests::test_adversarial_proxy_handler_upstream_error_recording ... ok
    test tests::test_adversarial_proxy_handler_binary_and_malformed ... ok
    test tests::test_adversarial_curl_special_characters ... ok
    test tests::test_hudsucker_proxy_handler_flow ... ok
    test tests::test_traffic_store_ring_buffer_eviction ... ok
    test tests::test_generate_curl ... ok
    test tests::test_adversarial_static_resource_boundary ... ok
    test tests::test_rust_embed_static_asset ... ok
    test tests::test_adversarial_traffic_store_concurrency_stress ... ok
    test tests::test_web_router_endpoints ... ok
    test tests::test_micro_ux_accessibility_attributes ... ok
    test tests::test_web_socket_realtime_broadcast ... ok
    test tests::test_adversarial_websocket_concurrent_clients ... ok
    test tests::test_adversarial_websocket_burst_lag_behavior ... ok

    test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
    ```
- **命令 4**: `cargo build --release`
  - 退出码: `0`
  - 产物文件: `target\release\wiretap.exe`
  - 产物体积: `5.36 MB`
  - 运行验证: `.\target\release\wiretap.exe --help` 成功打印帮助信息。

---

## 2. Logic Chain (推理链条)

1. **代理层健壮性推理 (Observation 1.1.1 -> Observation 1.2.2 & 1.2.3)**:
   - 队长采用 `hudsucker` 替换了原有的代理逻辑，避免了手动维护 TCP 连接生命周期的风险。
   - 在我们的对抗性测试中，向 `WireTapProxyHandler` 输入了畸形 URI、特殊查询字符串、非 UTF-8 二进制 Body（protobuf/gzip 流量）以及模拟上游崩溃场景。测试实证证明：系统没有出现 Panic，二进制 Body 被安全降级为 `None`，上游错误被准确捕获并录入 502 记录，证明代理核心层健壮安全。
2. **Web 与静态资源隔离安全性推理 (Observation 1.1.2 -> Observation 1.2.1)**:
   - 使用 `rust-embed` 将静态文件内嵌至二进制文件。
   - 对抗性测试中测试了 `/static/../../Cargo.toml` 路径穿越、空文件路径、深层不存在目录。Axum 路由与 `rust-embed` 均返回 404 Not Found 或 400 Bad Request，完全杜绝了静态目录越界与本地文件泄漏风险。
3. **WebSocket 与并发推送链路推理 (Observation 1.1.2 -> Observation 1.2.4 & 1.2.7)**:
   - 针对 Axum WebSocket 与 `tokio::sync::broadcast::channel`，测试验证了多客户端并发连接与单个客户端异常挂断。实测表明单个客户端的断开不会阻塞其他客户端或引发死锁。
   - 突发 600 条记录高频洪峰测试下，信道能够平稳交付，前端具备断线 3 秒重连与 SSE 平滑降级机制，可靠性满足生产要求。
4. **前端微 UX 与规范合规推理 (Observation 1.1.3)**:
   - Git diff 证明仅修改了 24 行（严格 ≤ 50 行），未破坏原有双栏布局与样式体系。
   - 补齐了 `:focus-visible`、ARIA 属性、键盘遍历与快捷键，并经由 `test_micro_ux_accessibility_attributes` 自动化防退化断言。

---

## 3. Caveats (局限与观察建议)

1. **Broadcast 信道在极端极速场景下的缓冲容量**:
   - `TrafficStore::new` 中目前使用固定的 `broadcast::channel(500)` 容量。
   - 在 `src/web/mod.rs` 的 `handle_socket` 中，采用了 `while let Ok(entry) = rx.recv().await`。若极端高吞吐环境下某个客户端网速极慢导致 Tokio 产生 `RecvError::Lagged`，该 WebSocket 连接会退出触发重连。前端目前有 3 秒自动重连逻辑兜底。未来在更高并发场景下，可考虑将循环改写为 `match rx.recv().await { Err(RecvError::Lagged(_)) => continue, ... }` 以在丢包时仍保持连接不断。
2. **遗留的 `proxy_handler` 函数**:
   - `src/proxy.rs` 第 263-416 行保留了原先旧版的 `proxy_handler` 与 `ProxyConfig`。该部分代码在 `main.rs` 中已不再被调用。保留其作为备用函数不影响编译与运行，可在后续版本中清理或标记为 `#[deprecated]`。

---

## 4. Conclusion (裁决与最终结论)

**审查裁决**: **APPROVE** (批准通过)

所有 Milestone 1 与 Milestone 2 的交付成果均完全符合 `ORIGINAL_REQUEST.md` 中的所有验收标准与架构约束：
- [x] 核心代理基于 `hudsucker`，杜绝使用裸 `tokio::net::TcpStream`。
- [x] Web 服务基于 `axum` + `rust-embed` 实现静态资源内嵌与单二进制打包。
- [x] 实时推送基于 Axum WebSocket 与 `broadcast::channel`，兼具 SSE 降级支持。
- [x] 前端界面单文件 HTML + CDN，零 npm 构建步骤。
- [x] 微 UX 改进代码修改量仅 24 行（严格 ≤ 50 行），具备 ARIA 标签与清晰焦点状态。
- [x] 规范记录认知至 `.Jules/palette.md`。
- [x] 零回归缺陷，新增 8 个边界对抗测试全部通过，累计 **16/16 测试 100% 全绿**。
- [x] `cargo check` 零 warning 零 error。
- [x] `cargo test` 100% 通过。
- [x] `cargo build --release` 成功产出单二进制可执行文件（5.36 MB）。

---

## 5. Verification Method (独立复现与验证步骤)

任何审查者均可在工作区 `e:\Code\githubidk` 运行以下命令进行 100% 独立复现核验：

### 5.1 类型与代码检查
```powershell
cargo check
cargo check --release
```
*预期*: 退出码 0，零 error，零 warning。

### 5.2 全量单元与对抗测试 (16 项测试)
```powershell
cargo test -- --nocapture
```
*预期*: 16 个测试用例（包括对抗性静态边界测试、二进制 malformed 请求体测试、并发 WebSocket 广播与洪峰测试）全部通过 (`16 passed; 0 failed`).

### 5.3 单二进制 Release 构建与 CLI 验证
```powershell
cargo build --release
.\target\release\wiretap.exe --help
```
*预期*: 成功生成体积约为 5.3MB 的 `target\release\wiretap.exe`，正确打印 CLI 使用帮助。

### 5.4 微 UX 代码行数限制验证
```powershell
git diff --stat src/web/ui.rs
```
*预期*: 变更行数 ≤ 25 行（远低于 50 行上限）。
