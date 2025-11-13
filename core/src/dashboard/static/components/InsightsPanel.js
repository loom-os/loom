import { html } from "../lib/deps.js";
import AgentRoster from "./AgentRoster.js";
import EventStream from "./EventStream.js";

function MetricsCard({ metrics, formatNumber, formattedFlowUpdatedAt }) {
  return html`
    <section class="card metrics-card">
      <div class="card-heading">
        <div>
          <h2>At a glance</h2>
          <p class="card-subtitle">Flow updated ${formattedFlowUpdatedAt}</p>
        </div>
      </div>
      <div class="metric-grid">
        <div class="metric">
          <span class="metric-label">Events / sec</span>
          <span class="metric-value"
            >${formatNumber(metrics.events_per_sec)}</span
          >
        </div>
        <div class="metric">
          <span class="metric-label">Active agents</span>
          <span class="metric-value"
            >${formatNumber(metrics.active_agents)}</span
          >
        </div>
        <div class="metric">
          <span class="metric-label">Subscriptions</span>
          <span class="metric-value"
            >${formatNumber(metrics.active_subscriptions)}</span
          >
        </div>
        <div class="metric">
          <span class="metric-label">Tool calls / sec</span>
          <span class="metric-value"
            >${formatNumber(metrics.tool_invocations_per_sec)}</span
          >
        </div>
      </div>
    </section>
  `;
}

function FiltersCard({ filters, setFilterValue }) {
  return html`
    <section class="card filters-card">
      <div class="card-heading">
        <h2>Filters</h2>
      </div>
      <label>
        Thread
        <input
          class="filter-input"
          type="text"
          value=${filters.threadId}
          onInput=${(event) => setFilterValue("threadId", event.target.value)}
          placeholder="thread-id"
        />
      </label>
      <label>
        Topic
        <input
          class="filter-input"
          type="text"
          value=${filters.topic}
          onInput=${(event) => setFilterValue("topic", event.target.value)}
          placeholder="agent.topic"
        />
      </label>
      <label>
        Sender
        <input
          class="filter-input"
          type="text"
          value=${filters.sender}
          onInput=${(event) => setFilterValue("sender", event.target.value)}
          placeholder="agent-id"
        />
      </label>
    </section>
  `;
}

export default function InsightsPanel({
  metrics,
  topology,
  filters,
  setFilterValue,
  autoScroll,
  toggleAutoScroll,
  clearEvents,
  filteredEvents,
  agents,
  selectedAgent,
  setSelectedAgent,
  agentTimeline,
  formatNumber,
  formatTime,
  formatRelativeTime,
  formattedFlowUpdatedAt,
  formattedTopologyUpdatedAt,
}) {
  return html`
    <aside class="insights-panel">
      <${MetricsCard}
        metrics=${metrics}
        formatNumber=${formatNumber}
        formattedFlowUpdatedAt=${formattedFlowUpdatedAt}
      />
      <${AgentRoster}
        topology=${topology}
        updatedAtLabel=${formattedTopologyUpdatedAt}
      />
      <${FiltersCard}
        filters=${filters}
        setFilterValue=${setFilterValue}
      />
      <${EventStream}
        autoScroll=${autoScroll}
        toggleAutoScroll=${toggleAutoScroll}
        clearEvents=${clearEvents}
        filteredEvents=${filteredEvents}
        agents=${agents}
        selectedAgent=${selectedAgent}
        setSelectedAgent=${setSelectedAgent}
        agentTimeline=${agentTimeline}
        formatTime=${formatTime}
        formatRelativeTime=${formatRelativeTime}
      />
    </aside>
  `;
}
