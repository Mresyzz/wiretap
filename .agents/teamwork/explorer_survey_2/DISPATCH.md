# Dispatch Task: Explorer 2 - Core Proxy & Realtime Backend Survey

Please read `e:\Code\githubidk\.agents\teamwork\ORIGINAL_REQUEST.md`.
Your role: Technical Explorer investigating the WireTap backend proxy implementation, MITM handling, Axum web server integration, and WebSocket broadcast channel.
Output your findings in `e:\Code\githubidk\.agents\teamwork\explorer_survey_2\handoff.md`.


## 2026-11-08T14:21:05Z
From: 1a23fc81-7715-4778-add1-e3a1ede39333
Investigate the core proxy and backend implementation of WireTap in e:\Code\githubidk:
1. Check `src/` to see how proxying is implemented: is it using `hudsucker` or `tokio::net::TcpStream` or something else?
2. Check how the web server is implemented: is `axum` used? How are static assets served (is `rust-embed` used)?
3. Check how traffic packets/events are forwarded to the web UI: is `tokio::sync::broadcast::channel` used with Axum WebSocket?
4. Document any architectural violations, existing design, and what needs to be changed in e:\Code\githubidk\.agents\teamwork\explorer_survey_2\handoff.md.
Send a message when you are done.
