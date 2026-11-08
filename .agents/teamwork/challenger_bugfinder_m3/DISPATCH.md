# Dispatch Task: 找 BUG (Bug Finder & Adversarial Tester) - Milestone 3

## 角色定义与提示词
您是【找 BUG】：负责审查代码改动、测试边界情况，寻找潜在缺陷并确认无功能回归。

## 任务目标
对 WireTap 开源项目经过队长（Milestone 1）与 Palette 🎨（Milestone 2）修改后的全部代码进行彻底的缺陷审查与边界回归验证：
1. **审查代码改动**：
   - 审查 `Cargo.toml` 依赖、`src/proxy.rs` (hudsucker 接入与 CA 生成)、`src/web/mod.rs` (rust-embed 与 /api/ws WebSocket 广播)、`static/index.html` 与 `src/web/ui.rs` (微 UX 与 ARIA)。
   - 检查是否存在内存泄漏、死锁、并发竞态、WebSocket 异常断连未释放、无效 URI 转发崩盘等缺陷。
   - 检查是否严格满足 ≤50 行微 UX 代码修改限制、零 npm 构建与单二进制规范。
2. **测试边界与极端情况**：
   - 编写对抗性/边界测试（例如极端大请求、畸形 HTTP 数据包、高频 WebSocket 广播、SSE 与 WebSocket 并发接入、静态资源不存在 404 等）。
   - 可在 `tests/unit_test.rs` 或新增测试文件中添加针对性边界测试。
3. **回归验证**：
   - 运行终端命令 `cargo check` 确保零错误。
   - 运行终端命令 `cargo test` 确保所有既有与新增测试用例全量 100% 通过。
   - 运行 `cargo build --release` 验证单二进制构建成功。
4. **输出结论**：
   - 是否发现潜在 Bug？如有，指出具体行数与场景。
   - 给出明确的裁决（APPROVE 或 REQUEST_CHANGES）。

## 关键诚信警示 (MANDATORY INTEGRITY WARNING)
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## 参考文档
- `e:\Code\githubidk\.agents\teamwork\ORIGINAL_REQUEST.md` (必读)
- `e:\Code\githubidk\PROJECT.md`
- `e:\Code\githubidk\.agents\teamwork\worker_leader_m1\handoff.md`
- `e:\Code\githubidk\.agents\teamwork\worker_palette_m2\handoff.md`

## 交付要求
请在 `e:\Code\githubidk\.agents\teamwork\challenger_bugfinder_m3\handoff.md` 输出详细的审查与测试交接报告，并在完成后发送消息。


## 2026-11-08T14:53:49Z
Received dispatch from parent (1a23fc81-7715-4778-add1-e3a1ede39333):
Your working directory is e:\Code\githubidk\.agents\teamwork\challenger_bugfinder_m3
Your task instructions are in e:\Code\githubidk\.agents\teamwork\challenger_bugfinder_m3\DISPATCH.md
You MUST read e:\Code\githubidk\.agents\teamwork\ORIGINAL_REQUEST.md before starting work.

Identity & Mission:
您是【找 BUG】：负责审查代码改动、测试边界情况，寻找潜在缺陷并确认无功能回归。
在 WireTap 中落地 Milestone 3：
1. 深入审查队长（M1）与 Palette（M2）引入的所有代码改动。
2. 针对 hudsucker 代理、Axum WebSocket、rust-embed 静态资源分发和前端 micro-UX 进行全方位边界对抗测试（可扩展 tests/unit_test.rs）。
3. 验证 `cargo check`、`cargo test`、`cargo build --release`，确认零回归缺陷。
4. 给出最终审查裁决（APPROVE 或 REQUEST_CHANGES）。
