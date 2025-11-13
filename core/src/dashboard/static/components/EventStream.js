import { React, html } from "../lib/deps.js";

function EventItem({ event, formatTime }) {
  return html`
    <article class="event-item">
      <header class="event-header">
        <span class="event-topic">${event.topic}</span>
        <span class="event-time">${formatTime(event.timestamp)}</span>
      </header>
      <div class="event-meta">
        <span class="event-type event-type-${event.event_type}">
          ${event.event_type}
        </span>
        ${event.sender
          ? html`
              <span class="event-meta-field">
                <span class="meta-label">sender</span>
                <span>${event.sender}</span>
              </span>
            `
          : null}
        ${event.thread_id
          ? html`
              <span class="event-meta-field">
                <span class="meta-label">thread</span>
                <span>${event.thread_id}</span>
              </span>
            `
          : null}
        ${event.correlation_id
          ? html`
              <span class="event-meta-field">
                <span class="meta-label">corr</span>
                <span>${event.correlation_id.slice(0, 8)}...</span>
              </span>
            `
          : null}
      </div>
      <pre class="event-payload">${event.payload_preview}</pre>
    </article>
  `;
}

function AgentTimeline({ selectedAgent, agentTimeline, formatTime, formatRelativeTime }) {
  if (!selectedAgent) {
    return null;
  }

  const lastSeen = agentTimeline.length > 0 ? agentTimeline[0].timestamp : null;

  return html`
    <section class="agent-timeline">
      <header class="timeline-header">
        <span>Timeline for <strong>${selectedAgent}</strong></span>
        <span>${lastSeen ? formatRelativeTime(lastSeen) : "n/a"}</span>
      </header>
      <div class="timeline-rows">
        ${agentTimeline.map(
          (evt) => html`
            <div class="timeline-row" key=${evt.event_id}>
              <span class="timeline-time">${formatTime(evt.timestamp)}</span>
              <span class="timeline-direction direction-${evt.direction}">
                ${evt.directionLabel}
              </span>
              <div>
                <div class="timeline-topic">${evt.topic}</div>
                <div class="timeline-preview">${evt.preview}</div>
              </div>
            </div>
          `
        )}
      </div>
    </section>
  `;
}

export default function EventStream({
  autoScroll,
  toggleAutoScroll,
  clearEvents,
  filteredEvents,
  agents,
  selectedAgent,
  setSelectedAgent,
  agentTimeline,
  formatTime,
  formatRelativeTime,
}) {
  const eventsListRef = React.useRef(null);

  React.useEffect(() => {
    if (autoScroll && eventsListRef.current) {
      eventsListRef.current.scrollTop = eventsListRef.current.scrollHeight;
    }
  }, [autoScroll, filteredEvents]);

  return html`
    <section class="card events-card">
      <div class="card-heading">
        <div>
          <h2>Event stream</h2>
          <p class="card-subtitle">
            ${filteredEvents.length} matching ${filteredEvents.length === 1 ? "event" : "events"}
          </p>
        </div>
        <div class="button-row">
          <button
            class="btn ${autoScroll ? "active" : ""}"
            onClick=${toggleAutoScroll}
          >
            ${autoScroll ? "Auto-scroll" : "Paused"}
          </button>
          <button class="btn btn-secondary" onClick=${clearEvents}>Clear</button>
        </div>
      </div>
      <div class="agent-controls">
        <button
          class="agent-control-btn ${selectedAgent ? "" : "active"}"
          onClick=${() => setSelectedAgent(null)}
        >
          All agents
        </button>
        ${agents.map(
          (agent) => html`
            <button
              key=${agent}
              class="agent-control-btn ${selectedAgent === agent ? "active" : ""}"
              onClick=${() =>
                setSelectedAgent(selectedAgent === agent ? null : agent)}
            >
              ${agent}
            </button>
          `
        )}
      </div>
      <div class="events-list" ref=${eventsListRef}>
        ${filteredEvents.length === 0
          ? html`<div class="empty-state">Waiting for events...</div>`
          : filteredEvents.map((event) =>
              html`<${EventItem} key=${event.event_id} event=${event} formatTime=${formatTime} />`
            )}
      </div>
      <${AgentTimeline}
        selectedAgent=${selectedAgent}
        agentTimeline=${agentTimeline}
        formatTime=${formatTime}
        formatRelativeTime=${formatRelativeTime}
      />
    </section>
  `;
}
