# WireTap

<p align="left">
  <strong>一款基于 Rust 构建的轻量级、零依赖、单二进制本地 HTTP/HTTPS 抓包调试与重放工作台。</strong>
</p>

<p align="left">
  <a href="README.md">English</a> | <a href="README_zh.md">简体中文</a>
</p>

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Binary Size](https://img.shields.io/badge/binary_size-%3C6MB-brightgreen.svg)](https://github.com/Mresyzz/wiretap/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blueviolet.svg)](#安装与运行)
[![Tests](https://img.shields.io/badge/tests-18%2F18%20passed-success.svg)](tests/unit_test.rs)

WireTap 将透明代理引擎与内置的现代化实时 Web 工作台深度融合。开发者无需安装 Node.js、Docker、Python 环境或注册任何云端账户，单文件直接运行即可在毫秒级内捕获、分析、筛选、修改并重放本地微服务与客户端网络流量。

```
                       +------------------------+
                       |   客户端 / 前端服务     |
                       +-----------+------------+
                                   | HTTP/HTTPS 请求 (:8080)
                                   v
+------------------------------------------------------------------------+
|                        WIRETAP (单二进制独立运行)                       |
|                                                                        |
|   +---------------------+                 +------------------------+   |
|   |   高性能透明代理     |--(转发请求)---->|   上游目标服务 / API    |   |
|   |      代理引擎       |<--(响应流回传)--|  (:3000 或 局域网服务)  |   |
|   +----------+----------+                 +------------------------+   |
|              |                                                         |
|              v (事件总线)                                               |
|   +---------------------+   WebSocket / SSE+-----------------------+   |
|   | 内存环形缓冲区       |---------------->| 内置 Web 调试控制台   |   |
|   | (上限 1000 条 FIFO) |<--(一键重放)-----| (:9090 极简触觉界面)  |   |
|   +---------------------+    修改与重放    +-----------------------+   |
+------------------------------------------------------------------------+
```

---

## 核心特性

- **纯正单二进制产物**：基于纯 Rust（Tokio、Axum、Hudsucker）构建，零外部运行时依赖，冷启动小于 5ms，常驻空闲内存占用小于 10MB。
- **本地优先与隐私保护**：所有捕获的请求头、报文体均保存在本地环形内存缓冲区中，不写磁盘临时垃圾，绝无任何遥测或强制登录。
- **In-Situ 原地重放与 Diff 比对**：在抓包详情抽屉中直接原地编辑 Method、Headers、Body 并重放，实时对比原始响应与重放响应的字段差异。
- **一键授权脱敏**：分享抓包数据前，一键自动模糊或遮蔽 `Authorization: Bearer`、`Cookie` 等敏感令牌与授权凭证。
- **本地微服务端口雷达**：内置最高 500 协程并发探测本地 `1-65535` 或常用开发端口，一键探测活跃 Web 端口并绑定为目标或监听。
- **系统代理无感接管 (`--sys`)**：通过 `--sys` 标志位在启动时自动设置为系统代理，退出时（`Ctrl+C`）通过 RAII Guard 自动恢复网络设置，彻底杜绝关掉抓包工具后断网的问题。
- **标准导入导出**：支持标准 cURL 命令直接导入执行；支持将当前抓包会话导出为标准 HAR 2.0 格式供其他工具分析。
- **零配置演示模式**：运行 `wiretap demo` 即可自动模拟生成真实的微服务实时抓包流，开箱即用。

---

## 快速上手

### 1. 体验演示模式
无需启动任何真实后端，直接体验完整的 Web 控制台与模拟数据流：
```bash
wiretap demo
```
在浏览器中打开 **`http://127.0.0.1:9090`**。

### 2. 反向代理模式
将 `:8080` 端口接收到的流量代理转发到本地 `:3000` 端口的服务：
```bash
wiretap :8080 --to http://localhost:3000
```

### 3. 系统代理模式
接管操作系统的全局 HTTP/HTTPS 代理流量（退出时自动复原系统网络）：
```bash
wiretap :8080 --to http://localhost:3000 --sys
```

### 4. Webhook / Mock 接收模式
用于无服务状态下接收并检查第三方 Webhook 回调或客户端请求：
```bash
wiretap mock :8080
```
所有到达 `:8080` 的请求均会自动返回 `200 OK` 并完整记录在 Web 控制台中。

---

## 安装与运行

### 1. 预编译单二进制下载
直接从 [GitHub Releases](https://github.com/Mresyzz/wiretap/releases) 下载对应平台的独立可执行文件，开箱即用，无需配置环境：
- **Windows (x86_64)**: `wiretap-windows-x86_64.exe`
- **Linux (x86_64)**: `wiretap-linux-x86_64`
- **macOS (Apple Silicon)**: `wiretap-darwin-arm64`
- **macOS (Intel)**: `wiretap-darwin-x86_64`

### 2. 使用 Cargo 安装
```bash
cargo install --git https://github.com/Mresyzz/wiretap.git
```

### 3. 从源码编译
环境要求：[Rust](https://www.rust-lang.org/) 1.75+

```bash
git clone https://github.com/Mresyzz/wiretap.git
cd wiretap

# 运行自动化测试 (18 项全通过)
cargo test

# 编译极致优化的 Release 二进制文件 (<6MB)
cargo build --release

# 编译产物位于:
./target/release/wiretap
```

---

## 命令行参数一览

```text
用法: wiretap [OPTIONS] [COMMAND]

子命令:
  demo    启动模拟微服务流量的零配置演示模式
  proxy   通过代理转发拦截并审查上游服务流量
  mock    启动独立 Mock / Webhook 接收模式
  help    显示帮助信息

选项:
      --listen <LISTEN>        代理监听端口 [默认: 8080]
      --to <TO>                要代理转发的上游目标 URL
      --ui-port <UI_PORT>      内置 Web 控制台端口 [默认: 9090]
      --max-history <HISTORY>  最大保留的内存请求记录条数 [默认: 1000]
  -s, --sys                    启动时自动配置系统代理，并在退出时自动还原
  -h, --help                   显示帮助
  -V, --version                显示版本
```

---

## 快捷键速查

| 快捷键 | 功能说明 |
| :--- | :--- |
| `Ctrl + K` 或 `/` | 聚焦搜索框并快速筛选 URL、路径或状态码 |
| `Space` (空格键) | 暂停 / 继续实时流量捕获 |
| `Alt + T` | 快速打开开发者工具箱与本地端口雷达 |
| `Ctrl + Enter` | 在原地重放抽屉打开时发送重放请求 |
| `Esc` | 关闭当前弹窗或重放抽屉 |

---

## 开源协议

本项目采用双重许可：

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

您可以根据需要选择任意一种许可进行使用。
