import { html } from "../lib/deps.js";

export default function AgentRoster({ topology, updatedAtLabel }) {
  const agents = topology.agents || [];
  const preview = agents.slice(0, 6);
  const remaining = Math.max(agents.length - preview.length, 0);

  return html`
    <section class="card topology-card">
      <div class="card-heading">
        <div>
          <h2>Agent roster</h2>
          <p class="card-subtitle">
            Updated ${updatedAtLabel}
          </p>
        </div>
        <span class="summary-chip">
          <span class="summary-label">Agents</span>
          <span class="summary-value">${agents.length}</span>
        </span>
      </div>
      ${agents.length === 0
        ? html`<div class="empty-state">No agents registered yet.</div>`
        : html`
            <ul class="topology-list">
              ${preview.map(
                (agent) => html`
                  <li class="topology-item" key=${agent.id}>
                    <div class="agent-name">${agent.id}</div>
                    <div class="agent-topics">
                      ${agent.topics.slice(0, 3).map(
                        (topic) => html`
                          <span class="topic-chip" key=${`${agent.id}-${topic}`}>
                            ${topic}
                          </span>
                        `
                      )}
                    </div>
                  </li>
                `
              )}
            </ul>
            ${remaining > 0
              ? html`<p class="more-indicator">+${remaining} more agents</p>`
              : null}
          `}
    </section>
  `;
}
