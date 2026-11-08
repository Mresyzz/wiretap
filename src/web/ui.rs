pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>WireTap Studio | Instant API Inspector & Replay</title>
  <style>
    :root {
      --bg-base: #0b0c0e;
      --bg-surface: #14161a;
      --bg-elevated: #1c1f24;
      --bg-hover: #262a32;
      --border-subtle: #23272f;
      --border-focus: #3b82f6;
      --text-main: #f3f4f6;
      --text-muted: #9ca3af;
      --text-dim: #6b7280;
      
      --accent-blue: #3b82f6;
      --accent-green: #10b981;
      --accent-amber: #f59e0b;
      --accent-red: #ef4444;
      --accent-purple: #8b5cf6;
      
      --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
      --font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background-color: var(--bg-base);
      color: var(--text-main);
      font-family: var(--font-sans);
      height: 100vh;
      display: flex;
      flex-direction: column;
      overflow: hidden;
      font-size: 13px;
    }

    /* Top Navigation Bar */
    header {
      background: var(--bg-surface);
      border-bottom: 1px solid var(--border-subtle);
      padding: 0 16px;
      height: 52px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      flex-shrink: 0;
    }
    .brand {
      display: flex;
      align-items: center;
      gap: 10px;
      font-weight: 700;
      font-size: 15px;
      letter-spacing: 0.5px;
    }
    .radar-pulse {
      width: 10px;
      height: 10px;
      background: var(--accent-green);
      border-radius: 50%;
      box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.7);
      animation: pulse 2s infinite;
    }
    @keyframes pulse {
      0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.7); }
      70% { transform: scale(1); box-shadow: 0 0 0 8px rgba(16, 185, 129, 0); }
      100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(16, 185, 129, 0); }
    }
    .stats-bar {
      display: flex;
      align-items: center;
      gap: 16px;
      background: var(--bg-base);
      padding: 4px 12px;
      border-radius: 6px;
      border: 1px solid var(--border-subtle);
      font-size: 12px;
    }
    .stat-item {
      display: flex;
      gap: 6px;
      align-items: center;
    }
    .stat-value {
      font-weight: 600;
      color: var(--text-main);
    }
    .actions {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .search-box {
      background: var(--bg-base);
      border: 1px solid var(--border-subtle);
      border-radius: 6px;
      padding: 6px 10px;
      color: var(--text-main);
      font-size: 12px;
      width: 220px;
      outline: none;
    }
    .search-box:focus { border-color: var(--border-focus); }
    .btn {
      background: var(--bg-elevated);
      color: var(--text-main);
      border: 1px solid var(--border-subtle);
      border-radius: 6px;
      padding: 6px 12px;
      font-size: 12px;
      font-weight: 500;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      transition: background 0.15s, border-color 0.15s;
    }
    .btn:hover { background: var(--bg-hover); border-color: var(--text-dim); }
    .btn-primary { background: var(--accent-blue); color: #fff; border-color: var(--accent-blue); }
    .btn-primary:hover { background: #2563eb; }

    /* Main Container: Split View */
    .main-container {
      display: flex;
      flex: 1;
      height: calc(100vh - 52px);
      overflow: hidden;
    }

    /* Left: Traffic Stream */
    .stream-panel {
      width: 480px;
      border-right: 1px solid var(--border-subtle);
      display: flex;
      flex-direction: column;
      background: var(--bg-surface);
      flex-shrink: 0;
    }
    .panel-header {
      padding: 10px 16px;
      border-bottom: 1px solid var(--border-subtle);
      font-weight: 600;
      font-size: 12px;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.5px;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    .stream-list {
      flex: 1;
      overflow-y: auto;
      list-style: none;
    }
    .stream-item {
      padding: 10px 14px;
      border-bottom: 1px solid var(--border-subtle);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      transition: background 0.1s;
    }
    .stream-item:hover { background: var(--bg-elevated); }
    .stream-item.active {
      background: var(--bg-elevated);
      border-left: 3px solid var(--accent-blue);
    }
    .item-left {
      display: flex;
      align-items: center;
      gap: 8px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .method-badge {
      font-family: var(--font-mono);
      font-size: 10px;
      font-weight: 700;
      padding: 2px 6px;
      border-radius: 4px;
      text-align: center;
      min-width: 48px;
    }
    .method-GET { background: rgba(16, 185, 129, 0.15); color: var(--accent-green); }
    .method-POST { background: rgba(59, 130, 246, 0.15); color: var(--accent-blue); }
    .method-PUT { background: rgba(245, 158, 11, 0.15); color: var(--accent-amber); }
    .method-DELETE { background: rgba(239, 68, 68, 0.15); color: var(--accent-red); }
    .method-PATCH { background: rgba(139, 92, 246, 0.15); color: var(--accent-purple); }

    .item-path {
      font-family: var(--font-mono);
      color: var(--text-main);
      font-size: 12px;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .item-right {
      display: flex;
      align-items: center;
      gap: 8px;
      flex-shrink: 0;
      font-size: 11px;
    }
    .status-badge {
      font-family: var(--font-mono);
      font-weight: 700;
      font-size: 11px;
      padding: 2px 6px;
      border-radius: 4px;
    }
    .status-2xx { color: var(--accent-green); background: rgba(16, 185, 129, 0.1); }
    .status-3xx { color: var(--accent-amber); background: rgba(245, 158, 11, 0.1); }
    .status-4xx { color: var(--accent-amber); background: rgba(245, 158, 11, 0.1); }
    .status-5xx { color: var(--accent-red); background: rgba(239, 68, 68, 0.15); }
    .duration-text {
      color: var(--text-dim);
      font-family: var(--font-mono);
    }
    .replay-tag {
      font-size: 9px;
      background: rgba(139, 92, 246, 0.2);
      color: var(--accent-purple);
      padding: 1px 4px;
      border-radius: 3px;
      font-weight: 600;
    }

    /* Right: Inspector Details */
    .inspector-panel {
      flex: 1;
      display: flex;
      flex-direction: column;
      background: var(--bg-base);
      overflow: hidden;
    }
    .empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100%;
      color: var(--text-dim);
      gap: 12px;
    }
    .empty-icon {
      font-size: 32px;
      opacity: 0.5;
    }

    .detail-view {
      display: flex;
      flex-direction: column;
      height: 100%;
    }
    .detail-header {
      padding: 12px 20px;
      background: var(--bg-surface);
      border-bottom: 1px solid var(--border-subtle);
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    .detail-url {
      font-family: var(--font-mono);
      font-size: 13px;
      color: var(--text-main);
      word-break: break-all;
    }
    .detail-actions {
      display: flex;
      gap: 8px;
      flex-shrink: 0;
    }
    .tabs-nav {
      display: flex;
      background: var(--bg-surface);
      border-bottom: 1px solid var(--border-subtle);
      padding: 0 16px;
    }
    .tab-btn {
      padding: 10px 16px;
      border: none;
      background: none;
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 600;
      cursor: pointer;
      border-bottom: 2px solid transparent;
      transition: all 0.15s;
    }
    .tab-btn:hover { color: var(--text-main); }
    .tab-btn.active {
      color: var(--accent-blue);
      border-bottom-color: var(--accent-blue);
    }
    .tab-content {
      flex: 1;
      overflow-y: auto;
      padding: 16px 20px;
    }

    .section-title {
      font-size: 12px;
      font-weight: 600;
      color: var(--text-dim);
      text-transform: uppercase;
      letter-spacing: 0.5px;
      margin-bottom: 8px;
    }
    .headers-table {
      width: 100%;
      border-collapse: collapse;
      font-family: var(--font-mono);
      font-size: 12px;
      margin-bottom: 20px;
    }
    .headers-table tr:hover { background: var(--bg-surface); }
    .headers-table td {
      padding: 6px 10px;
      border-bottom: 1px solid var(--border-subtle);
    }
    .header-key {
      color: var(--accent-blue);
      width: 200px;
      vertical-align: top;
    }
    .header-val {
      color: var(--text-muted);
      word-break: break-all;
    }

    .code-block {
      background: var(--bg-surface);
      border: 1px solid var(--border-subtle);
      border-radius: 6px;
      padding: 12px;
      font-family: var(--font-mono);
      font-size: 12px;
      color: #e5e7eb;
      white-space: pre-wrap;
      word-break: break-all;
      overflow-x: auto;
      line-height: 1.5;
    }
    .json-key { color: #60a5fa; }
    .json-string { color: #34d399; }
    .json-number { color: #f59e0b; }
    .json-boolean { color: #a78bfa; }
    .json-null { color: #9ca3af; }

    /* Flash toast */
    .toast {
      position: fixed;
      bottom: 20px;
      right: 20px;
      background: var(--bg-elevated);
      border: 1px solid var(--border-focus);
      color: var(--text-main);
      padding: 10px 18px;
      border-radius: 6px;
      font-size: 12px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.5);
      opacity: 0;
      transform: translateY(10px);
      transition: all 0.2s ease-in-out;
      pointer-events: none;
      z-index: 100;
    }
    .toast.show {
      opacity: 1;
      transform: translateY(0);
    }
  </style>
</head>
<body>
  <!-- Header -->
  <header>
    <div class="brand">
      <div class="radar-pulse"></div>
      <span>WIRETAP</span>
      <span style="font-size: 11px; color: var(--text-dim); font-weight: normal; margin-left: 4px;">Studio</span>
    </div>

    <div class="stats-bar">
      <div class="stat-item">
        <span style="color: var(--text-dim);">Total:</span>
        <span id="stat-total" class="stat-value">0</span>
      </div>
      <div class="stat-item">
        <span style="color: var(--text-dim);">Avg Latency:</span>
        <span id="stat-latency" class="stat-value">0 ms</span>
      </div>
      <div class="stat-item">
        <span style="color: var(--text-dim);">Errors:</span>
        <span id="stat-errors" class="stat-value" style="color: var(--accent-red);">0</span>
      </div>
    </div>

    <div class="actions">
      <input type="text" id="filter-input" class="search-box" placeholder="Filter /path or method (e.g. GET /api)..." />
      <button id="btn-clear" class="btn">Clear</button>
      <button id="btn-export" class="btn">Export JSON</button>
    </div>
  </header>

  <!-- Main View -->
  <div class="main-container">
    <!-- Left: Traffic List -->
    <div class="stream-panel">
      <div class="panel-header">
        <span>Captured Streams</span>
        <span id="stream-count" style="color: var(--text-dim); font-size: 11px;">0 items</span>
      </div>
      <ul id="stream-list" class="stream-list"></ul>
    </div>

    <!-- Right: Detail Inspector -->
    <div class="inspector-panel" id="inspector-panel">
      <div class="empty-state" id="empty-state">
        <div class="empty-icon">📡</div>
        <p>Select a request from the stream to inspect details & replay</p>
      </div>

      <div class="detail-view" id="detail-view" style="display: none;">
        <div class="detail-header">
          <div class="detail-url" id="detail-url"></div>
          <div class="detail-actions">
            <button id="btn-replay" class="btn btn-primary">⚡ Replay</button>
            <button id="btn-copy-curl" class="btn">📋 Copy cURL</button>
          </div>
        </div>

        <div class="tabs-nav">
          <button class="tab-btn active" data-tab="tab-resp">Response</button>
          <button class="tab-btn" data-tab="tab-req">Request</button>
          <button class="tab-btn" data-tab="tab-curl">cURL</button>
        </div>

        <div class="tab-content" id="tab-resp">
          <div style="display: flex; gap: 16px; margin-bottom: 16px;">
            <div>
              <div class="section-title">Status</div>
              <span id="detail-status" class="status-badge"></span>
            </div>
            <div>
              <div class="section-title">Latency</div>
              <span id="detail-duration" style="font-family: var(--font-mono); color: var(--text-main);"></span>
            </div>
            <div>
              <div class="section-title">Size</div>
              <span id="detail-size" style="font-family: var(--font-mono); color: var(--text-main);"></span>
            </div>
            <div>
              <div class="section-title">Time</div>
              <span id="detail-time" style="font-family: var(--font-mono); color: var(--text-dim);"></span>
            </div>
          </div>

          <div class="section-title">Response Headers</div>
          <table class="headers-table" id="resp-headers-table"></table>

          <div class="section-title" style="display: flex; justify-content: space-between; align-items: center;">
            <span>Response Body</span>
            <button id="btn-copy-resp-body" class="btn" style="padding: 2px 8px; font-size: 11px;">Copy</button>
          </div>
          <div class="code-block" id="resp-body-block"></div>
        </div>

        <div class="tab-content" id="tab-req" style="display: none;">
          <div class="section-title">Request Headers</div>
          <table class="headers-table" id="req-headers-table"></table>

          <div class="section-title" style="display: flex; justify-content: space-between; align-items: center;">
            <span>Request Body</span>
            <button id="btn-copy-req-body" class="btn" style="padding: 2px 8px; font-size: 11px;">Copy</button>
          </div>
          <div class="code-block" id="req-body-block"></div>
        </div>

        <div class="tab-content" id="tab-curl" style="display: none;">
          <div class="section-title">Generated cURL Command</div>
          <div class="code-block" id="curl-block"></div>
        </div>
      </div>
    </div>
  </div>

  <div id="toast" class="toast"></div>

  <script>
    let entries = [];
    let selectedId = null;
    let filterQuery = "";

    function showToast(msg) {
      const toast = document.getElementById("toast");
      toast.textContent = msg;
      toast.classList.add("show");
      setTimeout(() => toast.classList.remove("show"), 2000);
    }

    function formatJsonHighlight(jsonStr) {
      if (!jsonStr) return '<span style="color: var(--text-dim);">[Empty]</span>';
      try {
        const obj = JSON.parse(jsonStr);
        const formatted = JSON.stringify(obj, null, 2);
        return formatted
          .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
          .replace(/("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g, function (match) {
            let cls = 'json-number';
            if (/^"/.test(match)) {
              if (/:$/.test(match)) {
                cls = 'json-key';
              } else {
                cls = 'json-string';
              }
            } else if (/true|false/.test(match)) {
              cls = 'json-boolean';
            } else if (/null/.test(match)) {
              cls = 'json-null';
            }
            return `<span class="${cls}">${match}</span>`;
          });
      } catch (e) {
        return jsonStr.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
      }
    }

    function getStatusClass(code) {
      if (code >= 200 && code < 300) return 'status-2xx';
      if (code >= 300 && code < 400) return 'status-3xx';
      if (code >= 400 && code < 500) return 'status-4xx';
      return 'status-5xx';
    }

    function updateStats() {
      const total = entries.length;
      document.getElementById('stat-total').textContent = total;
      document.getElementById('stream-count').textContent = `${total} items`;

      if (total === 0) {
        document.getElementById('stat-latency').textContent = '0 ms';
        document.getElementById('stat-errors').textContent = '0';
        return;
      }

      let totalLatency = 0;
      let errors = 0;
      for (const e of entries) {
        totalLatency += e.duration_ms;
        if (e.status >= 400) errors++;
      }
      const avg = Math.round(totalLatency / total);
      document.getElementById('stat-latency').textContent = `${avg} ms`;
      document.getElementById('stat-errors').textContent = errors;
    }

    function renderStreamList() {
      const list = document.getElementById("stream-list");
      list.innerHTML = "";

      const filtered = entries.filter(e => {
        if (!filterQuery) return true;
        const q = filterQuery.toLowerCase();
        return e.path.toLowerCase().includes(q) || 
               e.method.toLowerCase().includes(q) || 
               e.status.toString().includes(q);
      });

      // 最新倒序
      for (let i = filtered.length - 1; i >= 0; i--) {
        const item = filtered[i];
        const li = document.createElement("li");
        li.className = `stream-item ${item.id === selectedId ? 'active' : ''}`;
        li.onclick = () => selectItem(item.id);

        const methodClass = `method-${item.method}`;
        const statusClass = getStatusClass(item.status);
        const replayTag = item.is_replay ? '<span class="replay-tag">REPLAY</span>' : '';

        li.innerHTML = `
          <div class="item-left">
            <span class="method-badge ${methodClass}">${item.method}</span>
            <span class="item-path" title="${item.path}">${item.path}</span>
          </div>
          <div class="item-right">
            ${replayTag}
            <span class="status-badge ${statusClass}">${item.status}</span>
            <span class="duration-text">${item.duration_ms}ms</span>
          </div>
        `;
        list.appendChild(li);
      }
    }

    function selectItem(id) {
      selectedId = id;
      renderStreamList();

      const item = entries.find(e => e.id === id);
      if (!item) return;

      document.getElementById("empty-state").style.display = "none";
      document.getElementById("detail-view").style.display = "flex";

      document.getElementById("detail-url").textContent = item.full_url;

      const statusBadge = document.getElementById("detail-status");
      statusBadge.textContent = `${item.status} ${item.status_text}`;
      statusBadge.className = `status-badge ${getStatusClass(item.status)}`;

      document.getElementById("detail-duration").textContent = `${item.duration_ms} ms`;
      document.getElementById("detail-size").textContent = `${item.response_size_bytes} B`;
      document.getElementById("detail-time").textContent = item.time_str;

      // Headers table
      const renderHeaders = (headers, tableId) => {
        const table = document.getElementById(tableId);
        if (!headers || headers.length === 0) {
          table.innerHTML = '<tr><td colspan="2" style="color: var(--text-dim);">No headers</td></tr>';
          return;
        }
        table.innerHTML = headers.map(h => `
          <tr>
            <td class="header-key">${h.key}</td>
            <td class="header-val">${h.value}</td>
          </tr>
        `).join('');
      };

      renderHeaders(item.response_headers, "resp-headers-table");
      renderHeaders(item.request_headers, "req-headers-table");

      document.getElementById("resp-body-block").innerHTML = formatJsonHighlight(item.response_body);
      document.getElementById("req-body-block").innerHTML = formatJsonHighlight(item.request_body);

      // cURL
      fetch(`/api/curl/${item.id}`)
        .then(r => r.text())
        .then(curl => {
          document.getElementById("curl-block").textContent = curl;
        })
        .catch(() => {});
    }

    // Tabs logic
    document.querySelectorAll(".tab-btn").forEach(btn => {
      btn.onclick = () => {
        document.querySelectorAll(".tab-btn").forEach(b => b.classList.remove("active"));
        document.querySelectorAll(".tab-content").forEach(c => c.style.display = "none");
        btn.classList.add("active");
        const tabId = btn.getAttribute("data-tab");
        document.getElementById(tabId).style.display = "block";
      };
    });

    // Replay button
    document.getElementById("btn-replay").onclick = () => {
      if (!selectedId) return;
      fetch(`/api/replay/${selectedId}`, { method: "POST" })
        .then(r => r.json())
        .then(res => {
          showToast("⚡ Replay dispatched successfully!");
        })
        .catch(err => {
          showToast("Replay failed: " + err);
        });
    };

    // Copy cURL
    document.getElementById("btn-copy-curl").onclick = () => {
      if (!selectedId) return;
      const curl = document.getElementById("curl-block").textContent;
      navigator.clipboard.writeText(curl).then(() => showToast("📋 cURL command copied!"));
    };

    document.getElementById("btn-copy-resp-body").onclick = () => {
      const item = entries.find(e => e.id === selectedId);
      if (item && item.response_body) {
        navigator.clipboard.writeText(item.response_body).then(() => showToast("Copied response body!"));
      }
    };

    document.getElementById("btn-copy-req-body").onclick = () => {
      const item = entries.find(e => e.id === selectedId);
      if (item && item.request_body) {
        navigator.clipboard.writeText(item.request_body).then(() => showToast("Copied request body!"));
      }
    };

    // Clear
    document.getElementById("btn-clear").onclick = () => {
      fetch("/api/clear", { method: "POST" }).then(() => {
        entries = [];
        selectedId = null;
        updateStats();
        renderStreamList();
        document.getElementById("detail-view").style.display = "none";
        document.getElementById("empty-state").style.display = "flex";
        showToast("Cleared captured streams");
      });
    };

    // Export
    document.getElementById("btn-export").onclick = () => {
      const blob = new Blob([JSON.stringify(entries, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `wiretap-traffic-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
      showToast("Exported traffic session");
    };

    // Filter
    document.getElementById("filter-input").oninput = (e) => {
      filterQuery = e.target.value;
      renderStreamList();
    };

    // SSE connection
    function connectSSE() {
      const eventSource = new EventSource("/api/events");
      eventSource.onmessage = (event) => {
        try {
          const entry = JSON.parse(event.data);
          entries.push(entry);
          updateStats();
          renderStreamList();

          // Auto select if first item
          if (!selectedId) {
            selectItem(entry.id);
          }
        } catch (e) {
          console.error("SSE parse error", e);
        }
      };
      eventSource.onerror = () => {
        console.warn("SSE disconnected, retrying...");
      };
    }

    // Initial load
    fetch("/api/list")
      .then(r => r.json())
      .then(data => {
        entries = data || [];
        updateStats();
        renderStreamList();
        if (entries.length > 0) {
          selectItem(entries[entries.length - 1].id);
        }
      })
      .catch(console.error);

    connectSSE();
  </script>
</body>
</html>
"#;
