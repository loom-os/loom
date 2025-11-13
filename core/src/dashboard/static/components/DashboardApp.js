import { React, html } from "../lib/deps.js";
import { useDashboardData } from "../hooks/useDashboardData.js";
import Header from "./Header.js";
import InsightsPanel from "./InsightsPanel.js";
import VisualPanel from "./VisualPanel.js";

const formatNumber = (value) => {
  if (value === undefined || value === null || Number.isNaN(value)) {
    return "0";
  }
  return new Intl.NumberFormat("en-US", {
    maximumFractionDigits: value < 10 ? 2 : 0,
  }).format(value);
};

const formatTime = (timestamp) => {
  if (!timestamp) {
    return "n/a";
  }
  return new Date(timestamp).toLocaleTimeString();
};

const formatRelativeTime = (timestamp) => {
  if (!timestamp) {
    return "n/a";
  }
  const parsed = Date.parse(timestamp);
  if (Number.isNaN(parsed)) {
    return "n/a";
  }
  const diffMs = Date.now() - parsed;
  if (diffMs < 1_000) {
    return "just now";
  }
  const diffSec = Math.round(diffMs / 1_000);
  if (diffSec < 60) {
    return `${diffSec}s ago`;
  }
  const diffMin = Math.round(diffSec / 60);
  if (diffMin < 60) {
    return `${diffMin}m ago`;
  }
  const diffHours = Math.round(diffMin / 60);
  if (diffHours < 24) {
    return `${diffHours}h ago`;
  }
  const diffDays = Math.round(diffHours / 24);
  return `${diffDays}d ago`;
};

export default function DashboardApp() {
  const {
    connectionStatus,
    eventCount,
    events,
    filteredEvents,
    agents,
    agentTimeline,
    autoScroll,
    toggleAutoScroll,
    clearEvents,
    metrics,
    topology,
    flowData,
    flowUpdatedAt,
    flowSummary,
    filters,
    setFilterValue,
    setSelectedAgent,
    selectedAgent,
  } = useDashboardData();

  const formattedFlowUpdatedAt = flowUpdatedAt
    ? new Date(flowUpdatedAt).toLocaleTimeString()
    : "n/a";
  const formattedTopologyUpdatedAt = topology.timestamp
    ? formatTime(topology.timestamp)
    : "n/a";

  return html`
    <div class="app-shell">
      <${Header}
        connectionStatus=${connectionStatus}
        eventCount=${eventCount}
      />
      <main class="dashboard-main">
        <${InsightsPanel}
          metrics=${metrics}
          topology=${topology}
          filters=${filters}
          setFilterValue=${setFilterValue}
          autoScroll=${autoScroll}
          toggleAutoScroll=${toggleAutoScroll}
          clearEvents=${clearEvents}
          filteredEvents=${filteredEvents}
          agents=${agents}
          selectedAgent=${selectedAgent}
          setSelectedAgent=${setSelectedAgent}
          agentTimeline=${agentTimeline}
          formatNumber=${formatNumber}
          formatTime=${formatTime}
          formatRelativeTime=${formatRelativeTime}
          formattedFlowUpdatedAt=${formattedFlowUpdatedAt}
          formattedTopologyUpdatedAt=${formattedTopologyUpdatedAt}
        />
        <${VisualPanel}
          flowData=${flowData}
          flowSummary=${flowSummary}
          formattedFlowUpdatedAt=${formattedFlowUpdatedAt}
          formatNumber=${formatNumber}
          formatTime=${formatTime}
          formatRelativeTime=${formatRelativeTime}
        />
      </main>
    </div>
  `;
}
