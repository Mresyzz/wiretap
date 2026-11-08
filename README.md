# 📡 WireTap

<p align="center">
  <strong>The zero-dependency, single-binary API traffic inspector, debugger, and replay studio.</strong><br>
  <em>Ditch the bloated cloud SaaS. Intercept, inspect, and replay HTTP traffic locally in milliseconds.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License" />
  <img src="https://img.shields.io/badge/binary_size-<8MB-success.svg" alt="Size" />
  <img src="https://img.shields.io/badge/dependencies-ZERO-orange.svg" alt="Zero Dependencies" />
  <img src="https://img.shields.io/badge/platform-macOS%20|%20Linux%20|%20Windows-blueviolet.svg" alt="Platforms" />
  <img src="https://img.shields.io/badge/cloud_account-NONE-red.svg" alt="No Account Required" />
</p>

---

```
                       ┌────────────────────────┐
                       │   Client / Frontend    │
                       └───────────┬────────────┘
                                   │ HTTP Req (:8080)
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        WIRETAP (Single Binary)                         │
│                                                                        │
│   ┌─────────────────────┐                 ┌────────────────────────┐   │
│   │  Transparent Proxy  │──(Upstream Req)─▶   Your Backend API     │   │
│   │       Engine        │◀─(Resp Metrics)─┤    (:3000 / Cloud)     │   │
│   └──────────┬──────────┘                 └────────────────────────┘   │
│              │                                                         │
│              ▼ (Event Bus)                                             │
│   ┌─────────────────────┐    SSE Stream    ┌───────────────────────┐   │
│   │ In-Memory RingBuffer│─────────────────▶│ Embedded Web Studio   │   │
│   │   (Max 1000 items)  │◀──(One-Click)───│ (:9090 Modern DarkUI) │   │
│   └─────────────────────┘    Replay Action └───────────────────────┘   │
└────────────────────────────────────────────────────────────────────────┘
```

---

## ⚡ Why WireTap?

For years, developers just wanted to inspect a local API request or test a webhook. Instead, commercial API tools succumbed to **severe service degradation (Enshittification)**:
- **Forced cloud login** & mandatory workspace synchronization.
- **Privacy leaks**: sending proprietary authorization tokens, customer payloads, and internal API keys to third-party cloud servers.
- **Bloatware**: Electron containers consuming 800MB+ RAM with 10-second startup splash screens.
- **Wireshark** is too low-level for application-layer JSON/REST; **Charles/Fiddler** are commercial nagware requiring messy root CA installation.

**WireTap fixes this with zero-compromise engineering:**
- 🚀 **Single Static Binary**: Built with pure Rust. No Node.js, no Python, no Docker, no external database. Starts in **<5ms** with **<10MB** RAM.
- 🔒 **100% Local & Private**: Zero accounts, zero cloud telemetry, zero remote dependencies. Everything stays in your memory.
- ⚡ **Zero-Config Transparent Proxy**: Run `wiretap :8080 --to localhost:3000`. Point your app to port 8080 and you're done. No SSL certificates to generate.
- 🎨 **Built-in Raycast-Grade Web Studio**: Modern dark-mode dashboard embedded right inside the executable. Live SSE waterfall, JSON syntax highlighting, latency breakdown, one-click replay, and instant cURL export.
- 🧪 **Zero-Setup Demo Mode**: Try it in 3 seconds without having any backend server running via `wiretap demo`.

---

## 📊 Feature Comparison

| Capability | WireTap | Postman / Insomnia | Charles / Fiddler | Wireshark |
| :--- | :---: | :---: | :---: | :---: |
| **Account / Login Required** | **None (Zero Account)** | Mandatory Cloud Sign-in | License Key / Paywall | None |
| **Data Privacy** | **100% Local (In-Memory)** | Cloud Sync by Default | Local | Local |
| **Binary Size & Runtime** | **< 8MB (Single Binary)** | ~350MB+ (Electron) | ~150MB+ (Java/.NET) | ~100MB+ |
| **Startup Time** | **< 5 ms** | 8 – 15 seconds | 4 – 8 seconds | ~2 seconds |
| **Root CA Installation** | **Not Needed (Proxy Mode)** | Not Needed | Mandatory for HTTPS | Not Needed (pcap) |
| **One-Click Replay** | **Yes (Built-in)** | Yes | Clunky | Complex |
| **Instant cURL Generation** | **Yes (1-Click Copy)** | Yes | Export required | No |
| **Resource Idle Cost** | **< 10MB RAM** | 600MB – 1.2GB RAM | 250MB – 400MB RAM | ~80MB RAM |
| **Open Source License** | **MIT / Apache-2.0** | Proprietary SaaS | Commercial Closed | GPLv2 |

---

## 🚀 Quick Start (Under 30 Seconds)

### 1. Instant Demo (No backend needed)
Experience WireTap immediately with live simulated microservice traffic:
```bash
wiretap demo
```
Open **`http://127.0.0.1:9090`** in your browser to watch requests stream in real time!

---

### 2. Transparent Proxy Mode
Forward traffic from `:8080` to your real backend running on `:3000`:
```bash
wiretap :8080 --to http://localhost:3000
```
- Send any request to `http://localhost:8080/api/...`
- WireTap forwards it to `http://localhost:3000/api/...` and records the complete payload, headers, and duration.
- Open **`http://127.0.0.1:9090`** to inspect the live waterfall and trigger one-click replays.

---

### 3. Mock / Sink Mode (Webhook & Callback Inspector)
Need to debug a third-party webhook (Stripe, GitHub, Shopify) or external script without writing a server?
```bash
wiretap mock :8080
```
WireTap will accept any incoming HTTP request (GET, POST, PUT, DELETE), return `200 OK`, and capture full headers and bodies for visual inspection.

---

## 🛠️ Installation

### Cargo (Recommended)
```bash
cargo install --path .
```

### Pre-built Binaries
Download pre-built standalone binaries directly from the repository's GitHub Releases tab:
- `wiretap-linux-x86_64`
- `wiretap-darwin-arm64` (Apple Silicon)
- `wiretap-windows-x86_64.exe`

Just place it anywhere in your `$PATH`. No runtime dependencies required.

---

## 💻 CLI Options

```text
Usage: wiretap [OPTIONS] [COMMAND]

Commands:
  demo    Launch zero-setup demo mode with live simulated traffic
  proxy   Intercept and inspect traffic by proxying to an upstream service
  mock    Run as a standalone mock/sink server to inspect webhooks or incoming requests
  help    Print this message or the help of the given subcommand(s)

Options:
  -l, --listen <LISTEN>            Listening port for the proxy [default: 8080]
  -t, --to <TO>                    Upstream target base URL to proxy to (e.g. http://localhost:3000)
      --ui-port <UI_PORT>          Port for the embedded Web Studio UI [default: 9090]
      --max-history <MAX_HISTORY>  Max traffic items in memory ring buffer [default: 1000]
  -h, --help                       Print help
  -V, --version                    Print version
```

---

## 🛡️ License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
