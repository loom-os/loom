import { React } from "../lib/deps.js";

const EVENT_BUFFER_LIMIT = 2000;

const parseJson = (text) => {
  try {
    return JSON.parse(text);
  } catch (err) {
    console.error("Failed to parse JSON payload", err);
    return null;
  }
};

export function useDashboardData() {
  const [connectionStatus, setConnectionStatus] = React.useState("Connecting...");
  const [eventCount, setEventCount] = React.useState(0);
  const [events, setEvents] = React.useState([]);
  const [metrics, setMetrics] = React.useState({
    events_per_sec: 0,
    active_agents: 0,
    active_subscriptions: 0,
    tool_invocations_per_sec: 0,
  });
  const [topology, setTopology] = React.useState({
    agents: [],
    edges: [],
    timestamp: null,
  });
  const [flowData, setFlowData] = React.useState({
    nodes: [],
    flows: [],
    timestamp: null,
  });
  const [flowUpdatedAt, setFlowUpdatedAt] = React.useState(null);
  const [autoScroll, setAutoScroll] = React.useState(true);
  const [filters, setFilters] = React.useState({
    threadId: "",
    topic: "",
    sender: "",
  });
  const [selectedAgent, setSelectedAgent] = React.useState(null);

  React.useEffect(() => {
    let cancel = false;
    let source;
    let reconnectTimer;

    const connect = () => {
      if (cancel) {
        return;
      }
      if (source) {
        source.close();
        source = undefined;
      }

      setConnectionStatus("Connecting...");
      source = new EventSource("/api/events/stream");

      source.onopen = () => {
        if (cancel) {
          return;
        }
        setConnectionStatus("Connected");
        if (reconnectTimer) {
          clearTimeout(reconnectTimer);
          reconnectTimer = undefined;
        }
      };

      source.onmessage = (event) => {
        if (cancel) {
          return;
        }
        const payload = parseJson(event.data);
        if (!payload) {
          return;
        }
        setEvents((prev) => {
          const next = prev.concat(payload);
          if (next.length > EVENT_BUFFER_LIMIT) {
            return next.slice(next.length - EVENT_BUFFER_LIMIT);
          }
          return next;
        });
        setEventCount((prev) => prev + 1);
      };

      source.onerror = () => {
        if (cancel) {
          return;
        }
        setConnectionStatus("Disconnected");
        if (source) {
          source.close();
          source = undefined;
        }
        if (!reconnectTimer) {
          reconnectTimer = setTimeout(connect, 3000);
        }
      };
    };

    connect();

    return () => {
      cancel = true;
      if (source) {
        source.close();
        source = undefined;
      }
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
      }
    };
  }, []);

  React.useEffect(() => {
    let cancel = false;

    const fetchTopology = async () => {
      try {
        const response = await fetch("/api/topology");
        const snapshot = await response.json();
        if (cancel) {
          return;
        }
        const agents = (snapshot.agents || []).map((agent) => ({
          ...agent,
          topics: agent.topics || [],
          capabilities: agent.capabilities || [],
        }));
        setTopology({
          ...snapshot,
          agents,
        });
        setMetrics((prev) => ({
          ...prev,
          active_agents: agents.length,
        }));
      } catch (err) {
        console.error("Failed to load topology", err);
      }
    };

    const fetchMetrics = async () => {
      try {
        const response = await fetch("/api/metrics");
        const snapshot = await response.json();
        if (cancel) {
          return;
        }
        setMetrics((prev) => ({
          ...prev,
          ...snapshot,
        }));
      } catch (err) {
        console.error("Failed to load metrics", err);
      }
    };

    const fetchFlow = async () => {
      try {
        const response = await fetch("/api/flow");
        const snapshot = await response.json();
        if (cancel) {
          return;
        }
        setFlowData({
          ...snapshot,
          nodes: snapshot.nodes || [],
          flows: snapshot.flows || [],
        });
        setFlowUpdatedAt(Date.now());
      } catch (err) {
        console.error("Failed to load flow graph", err);
      }
    };

    fetchTopology();
    fetchMetrics();
    fetchFlow();

    const topologyInterval = setInterval(fetchTopology, 10_000);
    const metricsInterval = setInterval(fetchMetrics, 2_000);
    const flowInterval = setInterval(fetchFlow, 2_500);

    return () => {
      cancel = true;
      clearInterval(topologyInterval);
      clearInterval(metricsInterval);
      clearInterval(flowInterval);
    };
  }, []);

  const setFilterValue = React.useCallback((key, value) => {
    setFilters((prev) => ({
      ...prev,
      [key]: value,
    }));
  }, []);

  const clearEvents = React.useCallback(() => {
    setEvents([]);
    setEventCount(0);
    setSelectedAgent(null);
  }, []);

  const toggleAutoScroll = React.useCallback(() => {
    setAutoScroll((prev) => !prev);
  }, []);

  const filteredEvents = React.useMemo(() => {
    return events.filter((evt) => {
      if (filters.threadId && !(evt.thread_id || "").includes(filters.threadId)) {
        return false;
      }
      if (filters.topic && !(evt.topic || "").includes(filters.topic)) {
        return false;
      }
      if (filters.sender && !(evt.sender || "").includes(filters.sender)) {
        return false;
      }
      if (selectedAgent && evt.sender !== selectedAgent) {
        return false;
      }
      return true;
    });
  }, [events, filters, selectedAgent]);

  const agents = React.useMemo(() => {
    const unique = new Set();
    for (const evt of events) {
      if (evt.sender) {
        unique.add(evt.sender);
      }
    }
    return Array.from(unique).sort();
  }, [events]);

  const agentTimeline = React.useMemo(() => {
    if (!selectedAgent) {
      return [];
    }
    return events
      .filter((evt) => evt.sender === selectedAgent)
      .map((evt) => {
        let direction = "system";
        if (evt.event_type === "event_delivered") {
          direction = "in";
        } else if (evt.event_type === "event_published") {
          direction = "out";
        }
        const previewRaw = (evt.payload_preview || "").trim();
        return {
          ...evt,
          direction,
          directionLabel:
            direction === "in"
              ? "In"
              : direction === "out"
              ? "Out"
              : "System",
          preview:
            previewRaw.length > 140
              ? `${previewRaw.slice(0, 137)}...`
              : previewRaw || "(no payload)",
        };
      })
      .sort((a, b) => {
        const ta = Date.parse(a.timestamp);
        const tb = Date.parse(b.timestamp);
        return tb - ta;
      });
  }, [events, selectedAgent]);

  const flowSummary = React.useMemo(() => {
    const nodes = flowData.nodes.length;
    const flows = flowData.flows.length;
    const topics = new Set(flowData.flows.map((f) => f.topic)).size;
    const activeLinks = flowData.flows.filter(
      (f) => Date.now() - f.last_event_ms < 4_000
    ).length;
    return {
      nodes,
      flows,
      topics,
      activeLinks,
    };
  }, [flowData]);

  return {
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
  };
}
