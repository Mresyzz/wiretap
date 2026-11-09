# Project Guidelines & CI/CD Rules

- **macOS CI Runner 选型**：严禁在 GitHub Actions 中使用已被官方弃用或缩容的 runner（如 `macos-12`, `macos-13`）；无论构建 `aarch64-apple-darwin` 还是 `x86_64-apple-darwin`，一律统一使用当前主力 `macos-latest` 进行原生或跨架构编译，避免调度排队卡死；Release 工作流必须附带 `workflow_dispatch` 允许手动触发。
- **版本与 Release 同步 (SemVer)**：完成新功能或重大特性合并时，必须主动同步递增项目版本号（如 `Cargo.toml`、CLI version 等）；涉及 Release 交付时，必须确保 Git Tag、代码版本与远程推送严格对齐，杜绝因漏打 Tag 导致 CI 依然构建旧版本的情况。
- **极简与 KISS 原则**：恪守 KISS 原则，零冗余第三方 C 库依赖，保持纯 Rust 单二进制架构；UI 界面严禁使用 Emoji，统一采用矢量 SVG 图标。
