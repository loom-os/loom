import { html } from "../lib/deps.js";

export default function Header({ connectionStatus, eventCount }) {
  const statusClass =
    connectionStatus === "Connected"
      ? "connected"
      : connectionStatus === "Connecting..."
      ? "pending"
      : "disconnected";

  return html`
    <header class="app-header">
      <div class="brand">
        <span class="logo-mark">◉</span>
        <div class="brand-text">
          <h1>Loom Dashboard</h1>
          <p>Real-time agent observability</p>
        </div>
      </div>
      <div class="status-cluster">
        <div class="status-pill ${connectionStatus !== "Connected"
          ? "muted"
          : ""}">
          <span class="status-dot ${statusClass}"></span>
          <span>${connectionStatus}</span>
        </div>
        <div class="status-pill muted">
          <div class="status-label">Events seen</div>
          <div class="status-value">${eventCount}</div>
        </div>
      </div>
    </header>
  `;
}
