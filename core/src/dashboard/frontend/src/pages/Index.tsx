import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { HeroSection } from "@/components/HeroSection";
import { MetricsOverview } from "@/components/MetricsOverview";
import { EventStream } from "@/components/EventStream";
import { AgentNetworkGraph } from "@/components/AgentNetworkGraph";
import {
  AgentCommunication,
  type Communication,
} from "@/components/AgentCommunication";
import { Button } from "@/components/ui/button";
import { Activity } from "lucide-react";
import {
  createEventStream,
  createSpansStream,
  fetchFlow,
  fetchMetrics,
  fetchTopology,
  normalizeMetrics,
  type DashboardEventPayload,
  type DashboardMetrics,
  type EventFlow as DashboardEventFlow,
  type FlowGraph,
  type FlowNode,
  type TopologySnapshot,
} from "@/lib/dashboardApi";

type QoSLabel = "Realtime" | "Batched" | "Background";

interface TimelineEvent {
  id: string;
  type: string;
  topic: string;
  sender: string;
  threadId?: string;
  correlationId?: string;
  timestamp: number;
  qos?: QoSLabel;
}

interface AgentNode {
  id: string;
  name: string;
  topics: string[];
  capabilities: string[];
  status: "active" | "idle" | "processing";
  connections: string[];
}

const MAX_EVENTS = 200;
const MAX_COMMUNICATIONS = 80;

const fallbackMetrics: DashboardMetrics = normalizeMetrics({});

const qosByEventType: Record<DashboardEventPayload["event_type"], QoSLabel> = {
  event_published: "Realtime",
  event_delivered: "Realtime",
  agent_registered: "Batched",
  agent_unregistered: "Background",
  tool_invoked: "Realtime",
  routing_decision: "Batched",
};

const communicationType: Record<
  DashboardEventPayload["event_type"],
  Communication["type"]
> = {
  event_published: "message",
  event_delivered: "message",
  agent_registered: "message",
  agent_unregistered: "message",
  tool_invoked: "tool_call",
  routing_decision: "output",
};

function toTimelineEvent(payload: DashboardEventPayload): TimelineEvent {
  return {
    id: payload.event_id,
    type: payload.event_type.replace("_", "."),
    topic: payload.topic,
    sender: payload.sender ?? "system",
    threadId: payload.thread_id,
    correlationId: payload.correlation_id,
    timestamp: Date.parse(payload.timestamp),
    qos: qosByEventType[payload.event_type] ?? "Realtime",
  };
}

function toCommunication(payload: DashboardEventPayload): Communication {
  const type = communicationType[payload.event_type] ?? "message";
  const base: Communication = {
    id: `${payload.event_id}-${payload.event_type}`,
    timestamp: Date.parse(payload.timestamp),
    agent: payload.sender ?? "system",
    type,
    content:
      payload.payload_preview?.length > 0
        ? payload.payload_preview
        : `${payload.topic} (${payload.event_type})`,
    threadId: payload.thread_id,
  };

  if (type === "tool_call") {
    return {
      ...base,
      tool: payload.topic,
      result: payload.correlation_id
        ? `corr=${payload.correlation_id}`
        : undefined,
    };
  }

  return base;
}

function statusFromNode(node: FlowNode | undefined, now: number) {
  if (!node) {
    return "idle" as const;
  }

  const delta = Math.max(0, now - node.last_active_ms);

  if (delta < 5_000) {
    return "active" as const;
  }
  if (delta < 30_000) {
    return "processing" as const;
  }
  return "idle" as const;
}

function deriveAgentGraph(
  topology?: TopologySnapshot,
  flow?: FlowGraph,
): {
  agents: AgentNode[];
  messages: Array<{ from: string; to: string; timestamp: number }>;
} {
  if (!topology) {
    return { agents: [], messages: [] };
  }

  const now = Date.now();
  const nodeMap = new Map(flow?.nodes.map((node) => [node.id, node]));

  const isAgentNode = (id: string) =>
    nodeMap.get(id)?.node_type === "agent" ||
    topology.agents.some((agent) => agent.id === id);

  const agents: AgentNode[] = topology.agents.map((agent) => {
    const flowNode = nodeMap.get(agent.id);
    const directConnections = (flow?.flows ?? [])
      .filter((edge) => edge.source === agent.id && isAgentNode(edge.target))
      .map((edge) => edge.target);

    const uniqueConnections = Array.from(new Set(directConnections));

    return {
      id: agent.id,
      name: agent.id,
      topics: agent.topics,
      capabilities: agent.capabilities,
      status: statusFromNode(flowNode, now),
      connections: uniqueConnections,
    };
  });

  const messages =
    flow?.flows
      .filter(
        (edge) => isAgentNode(edge.source) && isAgentNode(edge.target),
      )
      .map((edge) => ({
        from: edge.source,
        to: edge.target,
        timestamp: edge.last_event_ms ?? now,
      })) ?? [];

  return { agents, messages };
}

function foldEvents(
  list: TimelineEvent[],
  incoming: TimelineEvent,
  limit: number,
): TimelineEvent[] {
  const existing = list.some((evt) => evt.id === incoming.id);
  if (existing) {
    return list;
  }
  return [...list, incoming].slice(-limit);
}

function foldCommunications(
  list: Communication[],
  incoming: Communication,
  limit: number,
): Communication[] {
  const existing = list.some((comm) => comm.id === incoming.id);
  if (existing) {
    return list;
  }
  return [...list, incoming].slice(-limit);
}

const Index = () => {
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [communications, setCommunications] = useState<Communication[]>([]);
  const [llmCommunications, setLlmCommunications] = useState<Communication[]>([]);
  const [eventsPerWindow, setEventsPerWindow] = useState<number[]>([]);

  const { data: metricsData } = useQuery({
    queryKey: ["metrics"],
    queryFn: fetchMetrics,
    refetchInterval: 5_000,
    staleTime: 4_000,
  });

  const { data: topologyData } = useQuery({
    queryKey: ["topology"],
    queryFn: fetchTopology,
    refetchInterval: 10_000,
    staleTime: 8_000,
  });

  const { data: flowData } = useQuery({
    queryKey: ["flow"],
    queryFn: fetchFlow,
    refetchInterval: 3_000,
    staleTime: 2_500,
  });

  // Stream Dashboard events for Event Stream panel and lightweight communications fallback
  useEffect(() => {
    let source: EventSource | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
    let isMounted = true;

    const handleMessage = (event: MessageEvent<string>) => {
      if (!event.data || !isMounted) {
        return;
      }

      try {
        const parsed = JSON.parse(event.data) as DashboardEventPayload;
        const timelineEvent = toTimelineEvent(parsed);
        const communication = toCommunication(parsed);

        setEvents((prev) => foldEvents(prev, timelineEvent, MAX_EVENTS));
        setCommunications((prev) =>
          foldCommunications(prev, communication, MAX_COMMUNICATIONS),
        );
        setEventsPerWindow((prev) =>
          [...prev, timelineEvent.timestamp].slice(-MAX_EVENTS),
        );
      } catch (error) {
        console.warn("Failed to parse dashboard event", error);
      }
    };

    const connect = () => {
      if (!isMounted) return;

      try {
        source = createEventStream();

        source.onmessage = handleMessage;

        source.onerror = () => {
          if (!isMounted) return;

          console.warn("SSE connection error, reconnecting in 2s...");
          if (source) {
            source.close();
            source = null;
          }

          if (reconnectTimer) {
            clearTimeout(reconnectTimer);
          }

          reconnectTimer = setTimeout(() => {
            if (isMounted) {
              connect();
            }
          }, 2000);
        };
      } catch (error) {
        console.error("Failed to create EventSource:", error);
      }
    };

    connect();

    return () => {
      isMounted = false;
      if (source) {
        source.close();
        source = null;
      }
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
      }
    };
  }, []);

  // Stream spans for LLM outputs (Agent Communication)
  useEffect(() => {
    let source: EventSource | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
    let isMounted = true;

    const handleSpans = (event: MessageEvent<string>) => {
      if (!event.data || !isMounted) return;
      try {
        const spans = JSON.parse(event.data) as Array<{
          trace_id: string;
          span_id: string;
          name: string;
          start_time: number; // ns
          attributes: Record<string, string>;
          status: string;
        }>;

        const newItems: Communication[] = spans
          .filter((s) => s.attributes?.capability === "llm.generate" || s.name === "llm.generate")
          .map((s) => {
            const agent = s.attributes?.sender || s.attributes?.["loom.sender"] || "agent";
            const content =
              s.attributes?.output_preview || s.attributes?.["llm.output_preview"] || "";
            const threadId = s.attributes?.thread_id;
            const model = s.attributes?.model;
            const provider = s.attributes?.provider;

            const toolLabel = provider && model ? `${provider}:${model}` : model || provider || "llm";

            const item: Communication = {
              id: s.span_id,
              timestamp: Math.floor((s.start_time || 0) / 1_000_000),
              agent,
              type: "output",
              content: content && content.length > 0 ? content : s.name,
              tool: toolLabel,
              threadId,
            };
            return item;
          });

        if (newItems.length > 0) {
          setLlmCommunications((prev) => {
            const merged = [...prev, ...newItems];
            // de-dup by id and cap length
            const map = new Map<string, Communication>();
            for (const c of merged.slice(-MAX_COMMUNICATIONS * 2)) {
              map.set(c.id, c);
            }
            return Array.from(map.values()).slice(-MAX_COMMUNICATIONS);
          });
        }
      } catch (err) {
        console.warn("Failed to parse spans payload", err);
      }
    };

    const connect = () => {
      if (!isMounted) return;
      try {
        source = createSpansStream();

        source.addEventListener("spans", handleSpans as any);

        source.onerror = () => {
          if (!isMounted) return;
          if (source) {
            source.close();
            source = null;
          }
          if (reconnectTimer) clearTimeout(reconnectTimer);
          reconnectTimer = setTimeout(connect, 2000);
        };
      } catch (error) {
        console.error("Failed to create Spans EventSource:", error);
      }
    };

    connect();
    return () => {
      isMounted = false;
      if (source) source.close();
      if (reconnectTimer) clearTimeout(reconnectTimer);
    };
  }, []);

  const graph = useMemo(
    () => deriveAgentGraph(topologyData, flowData),
    [topologyData, flowData],
  );

  const recentMessages = useMemo(
    () => graph.messages.slice(-20),
    [graph.messages],
  );

  const metrics = useMemo(() => {
    const base = metricsData ?? fallbackMetrics;

    if (eventsPerWindow.length < 2) {
      return base;
    }

    const sorted = [...eventsPerWindow].sort((a, b) => a - b);
    const spanMs =
      sorted[sorted.length - 1]! - sorted[0]! || 1;
    const rate = (sorted.length / spanMs) * 1_000;

    return {
      ...base,
      eventsPerSecond: parseFloat(rate.toFixed(2)),
      activeAgents: graph.agents.length || base.activeAgents,
    };
  }, [metricsData, eventsPerWindow, graph.agents.length]);

  return (
    <div className="min-h-screen bg-background">
      <div className="container mx-auto px-4 py-8">
        <HeroSection />

        {/* Timeline Link */}
        <div className="mb-6 flex justify-end">
          <Link to="/timeline">
            <Button variant="outline" className="gap-2">
              <Activity className="h-4 w-4" />
              View Trace Timeline
            </Button>
          </Link>
        </div>

        <div className="space-y-8">
          <MetricsOverview metrics={metrics} />

          <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
            <EventStream events={events} />
            <AgentCommunication communications={llmCommunications.length > 0 ? llmCommunications : communications} />
          </div>

          <AgentNetworkGraph agents={graph.agents} messages={recentMessages} />
        </div>

        <footer className="mt-16 border-t border-border/30 py-6 text-center">
          <p className="text-sm text-muted-foreground">
            Loom — Weaving Intelligence into the Fabric of Reality
          </p>
          <p className="mt-2 text-xs text-muted-foreground">
            Event-Driven AI OS • Multi-Agent Collaboration • Smart Routing
          </p>
        </footer>
      </div>
    </div>
  );
};

export default Index;
