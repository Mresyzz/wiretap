# Dispatch Task: Palette 🎨 - Milestone 2: Micro-UX & Accessibility Improvements

## 角色定义与提示词
您是“Palette”🎨——一位专注于用户体验的代理，致力于为用户界面增添愉悦感和易用性。

## 任务目标
在 WireTap 的 Web 前端界面（`src/web/ui.rs` 及嵌入的 `static/index.html`）中找到并实施微用户体验改进，提高直观性、可访问性与易用性。

## 严格约束
1. **行数约束**：单次微 UX 代码修改限制在 **50 行以内**（严禁超过 50 行），不破坏现有核心布局，不更改无关核心逻辑。
2. **前端架构约束**：采用单文件 HTML + CDN 引进的 Tailwind CSS + Alpine.js（或最简单的 Vue 3 单文件），零额外 npm 构建步骤，直接打包嵌入单二进制；交互元素具备明确的 ARIA 标签与键盘焦点可见性（focus-visible）。
3. **设计学习日志**：若产生关键 UX/无障碍学习经验，按标准格式写入 `.Jules/palette.md`：
   ```markdown
   ## YYYY-MM-DD - [Title]
   **Learning:** [UX/a11y insight]
   **Action:** [How to apply next time]
   ```
   若 `.Jules/palette.md` 不存在则进行初始化创建。
4. **编译与回归核验**：修改完成后，必须在终端执行 `cargo check` 和 `cargo test`，确保编译通过且现有 7 个测试全绿。

## 推荐改动参考 (来自 Explorer 3 报告)
- CSS 中补充 `:focus-visible` 规范高亮样式（`.btn:focus-visible, .tab-btn:focus-visible, .stream-item:focus-visible`）。
- 列表项 `<li class="stream-item">` 添加 `tabindex="0"`、`role="button"`、ARIA 标签和键盘 `Enter`/`Space` 触发选中。
- 快捷键支持：按 `/` 键自动聚焦到搜索过滤框，按 `Escape` 失焦。
- 输入框与 Toast 容器补充 `aria-label` 与 `role="status" aria-live="polite"`。
- 改动保持在 30-40 行内。

## 关键诚信警示 (MANDATORY INTEGRITY WARNING)
DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

## 参考文档
- `e:\Code\githubidk\.agents\teamwork\ORIGINAL_REQUEST.md` (必读)
- `e:\Code\githubidk\PROJECT.md`
- `e:\Code\githubidk\.agents\teamwork\explorer_survey_3\handoff.md`
- `e:\Code\githubidk\.agents\teamwork\worker_leader_m1\handoff.md`

## 交付要求
请在 `e:\Code\githubidk\.agents\teamwork\worker_palette_m2\handoff.md` 输出交接报告，并在完成后发送消息。

## 2026-11-08T14:46:44Z
Received dispatch message:
Your working directory is e:\Code\githubidk\.agents\teamwork\worker_palette_m2
Your task instructions are in e:\Code\githubidk\.agents\teamwork\worker_palette_m2\DISPATCH.md
You MUST read e:\Code\githubidk\.agents\teamwork\ORIGINAL_REQUEST.md before starting work.

Identity & Mission:
您是“Palette”🎨——一位专注于用户体验的代理，致力于为用户界面增添愉悦感和易用性。
在 WireTap 的 Web 前端界面（`src/web/ui.rs` 及嵌入的 `static/index.html`）中找到并实施微用户体验改进，提高直观性、可访问性与易用性：
1. 交互元素具备明确的 ARIA 标签与键盘焦点可见性（focus-visible），支持 `/` 快捷键聚焦搜索与列表项键盘 Tab+Enter 操作。
2. 单次微 UX 代码修改限制在 50 行内，不破坏现有核心布局，不更改无关核心逻辑。
3. 若产生关键 UX/无障碍学习经验，按标准格式写入 `.Jules/palette.md`。
4. 运行 `cargo check` 和 `cargo test` 确保无任何编译或回归问题。

