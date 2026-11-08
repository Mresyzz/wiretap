# WireTap

<p align="left">
  <strong>A lightweight, zero-dependency, single-binary HTTP/HTTPS traffic inspector and replay studio written in Rust.</strong>
</p>

<p align="left">
  <a href="README.md">English</a> | <a href="README_zh.md">简体中文</a>
</p>

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Binary Size](https://img.shields.io/badge/binary_size-%3C6MB-brightgreen.svg)](https://github.com/Mresyzz/wiretap/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blueviolet.svg)](#installation)
[![Tests](https://img.shields.io/badge/tests-18%2F18%20passed-success.svg)](tests/unit_test.rs)

WireTap integrates a transparent proxy engine with an embedded real-time web workbench, allowing developers to capture, inspect, filter, modify, and replay local or microservice API traffic without external dependencies, Node.js runtimes, or cloud accounts.

```
                       +------------------------+
                       |   Client / Frontend    |
                       +-----------+------------+
                                   | HTTP/HTTPS Req (:8080)
                                   v
+------------------------------------------------------------------------+
|                        WIRETAP (Single Binary)                         |
|                                                                        |
|   +---------------------+                 +------------------------+   |
|   |  Transparent Proxy  |--(Forward Req)->|   Your Backend API     |   |
|   |       Engine        |<--(Resp Stream)-|    (:3000 / Cloud)     |   |
|   +----------+----------+                 +------------------------+   |
|              |                                                         |
|              v (Event Bus)                                             |
|   +---------------------+   WebSocket / SSE+-----------------------+   |
|   | In-Memory RingBuffer|----------------->| Embedded Web Studio   |   |
|   |   (Max 1000 items)  |<--(One-Click)----| (:9090 Tactile UI)    |   |
|   +---------------------+    Replay Action +-----------------------+   |
+------------------------------------------------------------------------+
```

---

## Core Capabilities

- **Single Static Binary**: Built with pure Rust (Tokio, Axum, Hudsucker). No Docker, Python, or Node.js required. Starts in under 5 ms with less than 10 MB idle RAM.
- **Local-First & Zero Telemetry**: Captured headers and payloads remain strictly in local memory. Zero cloud telemetry, mandatory logins, or tracking.
- **In-Situ Replay & Diff**: Edit method, headers, or body directly in the inspector drawer, re-execute the request instantly, and review side-by-side Response Diffs.
- **Credential Masking**: Automatically detects and redacts `Authorization: Bearer`, `Cookie`, and tokens with one click before sharing captures.
- **Local Port Radar**: High-concurrency engine (up to 500 tasks) that scans `1-65535` or common development ports to discover active microservices and bind them with one click.
- **Auto System Proxy (`--sys`)**: Automatically configures the OS proxy on launch and safely restores network settings upon exit (`Ctrl+C`).
- **Standard Import / Export**: One-click import and execution of raw cURL commands; export captured traffic sessions to standard HAR 2.0 format.
- **Zero-Setup Demo**: Run `wiretap demo` to explore the complete studio with live simulated microservice traffic.

---

## Quick Start

### 1. Demo Mode
Explore the interface with simulated microservice traffic:
```bash
wiretap demo
```
Open **`http://127.0.0.1:9090`** in your browser.

### 2. Reverse Proxy Mode
Forward traffic from `:8080` to an upstream backend on `:3000`:
```bash
wiretap :8080 --to http://localhost:3000
```

### 3. System Proxy Mode
Automatically route all system HTTP/HTTPS requests through WireTap (automatically restored on exit):
```bash
wiretap :8080 --to http://localhost:3000 --sys
```

### 4. Webhook / Mock Sink Mode
Capture third-party callbacks or webhooks without running an upstream server:
```bash
wiretap mock :8080
```
All incoming requests return `200 OK` and are recorded for real-time inspection.

---

## Command-Line Usage

```text
Usage: wiretap [OPTIONS] [COMMAND]

Commands:
  demo    Launch zero-setup demo mode with live simulated streams
  proxy   Intercept and inspect traffic by proxying to an upstream service
  mock    Run as a standalone mock/sink server to inspect webhooks
  help    Print this message or the help of the given subcommand(s)

Options:
      --listen <LISTEN>        Listening port for the proxy [default: 8080]
      --to <TO>                Upstream target base URL to proxy to
      --ui-port <UI_PORT>      Port for the embedded Web Studio UI [default: 9090]
      --max-history <HISTORY>  Maximum traffic entries to retain [default: 1000]
  -s, --sys                    Automatically configure OS proxy on launch and restore on exit
  -h, --help                   Print help
  -V, --version                Print version
```

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + K` or `/` | Focus search and URL/method filter |
| `Space` | Pause / resume real-time traffic capture |
| `Alt + T` | Open Developer Toolbox & Port Radar |
| `Ctrl + Enter` | Send replay request when drawer is open |
| `Esc` | Close modal or replay drawer |

---

## Build from Source

Requirements: [Rust](https://www.rust-lang.org/) 1.75+

```bash
# Clone the repository
git clone https://github.com/Mresyzz/wiretap.git
cd wiretap

# Run tests (18 tests)
cargo test

# Build optimized release binary (<6MB)
cargo build --release

# The compiled binary will be available at:
./target/release/wiretap
```

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
