const runtimeBase =
  typeof window !== "undefined" ? window.location.origin : undefined;

const CONFIGURED_BASE = (
  import.meta.env.VITE_DASHBOARD_API_BASE as string | undefined
)?.trim();

const API_BASE =
  CONFIGURED_BASE && CONFIGURED_BASE.length > 0
    ? CONFIGURED_BASE.replace(/\/+$/, "")
    : runtimeBase;

function buildUrl(path: string): string {
  if (path.startsWith("http://") || path.startsWith("https://")) {
    return path;
  }
  if (!API_BASE) {
    return path;
  }

  try {
    return new URL(path, API_BASE).toString();
  } catch (_) {
    return path;
  }
}

async function fetchJson<T>(path: string): Promise<T> {
  const response = await fetch(buildUrl(path), {
    headers: {
      Accept: "application/json",
    },
    credentials: "omit",
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch ${path}: ${response.status}`);
  }

  const text = await response.text();

  try {
    return JSON.parse(text) as T;
  } catch (error) {
    throw new Error(
      `Failed to parse JSON for ${path}: ${(error as Error).message}`,
    );
  }
}

export interface DashboardMetricsResponse {
  events_per_sec?: number;
  active_agents?: number;
  active_subscriptions?: number;
  routing_decisions?: number;
  average_latency_ms?: number;
  tool_invocations_per_sec?: number;
  qos_breakdown?: {
    realtime?: number;
    batched?: number;
    background?: number;
  };
}

export interface DashboardMetrics {
  eventsPerSecond: number;
  activeAgents: number;
  routingDecisions: number;
  averageLatencyMs: number;
  toolInvocationsPerSecond: number;
  qosBreakdown: {
    realtime: number;
    batched: number;
    background: number;
  };
}

export interface FlowNode {
  id: string;
  node_type: "agent" | "eventbus" | "router" | "llm" | "tool" | "storage";
  event_count: number;
  topics: string[];
  last_active_ms: number;
}

export interface EventFlow {
  source: string;
  target: string;
  topic: string;
  count: number;
  last_event_ms: number;
}

export interface FlowGraph {
  nodes: FlowNode[];
  flows: EventFlow[];
  timestamp: string;
}

export interface TopologyAgent {
  id: string;
  topics: string[];
  capabilities: string[];
}

export interface TopologyEdge {
  from_topic: string;
  to_agent: string;
  event_count: number;
}

export interface TopologySnapshot {
  agents: TopologyAgent[];
  edges: TopologyEdge[];
  timestamp: string;
}

export interface DashboardEventPayload {
  timestamp: string;
  event_type:
    | "event_published"
    | "event_delivered"
    | "agent_registered"
    | "agent_unregistered"
    | "tool_invoked"
    | "routing_decision";
  event_id: string;
  topic: string;
  sender?: string;
  thread_id?: string;
  correlation_id?: string;
  payload_preview: string;
}

function normalizeBreakdown(
  breakdown?: DashboardMetricsResponse["qos_breakdown"],
) {
  const realtime = breakdown?.realtime ?? 0;
  const batched = breakdown?.batched ?? 0;
  const background = breakdown?.background ?? 0;

  if (realtime + batched + background > 0) {
    return { realtime, batched, background };
  }

  // Provide a reasonable default split if server does not supply metrics.
  return { realtime: 60, batched: 30, background: 10 };
}

export function normalizeMetrics(
  raw: DashboardMetricsResponse,
): DashboardMetrics {
  return {
    eventsPerSecond: raw.events_per_sec ?? 0,
    activeAgents: raw.active_agents ?? 0,
    routingDecisions:
      raw.routing_decisions ?? raw.active_subscriptions ?? 0,
    averageLatencyMs: raw.average_latency_ms ?? 0,
    toolInvocationsPerSecond: raw.tool_invocations_per_sec ?? 0,
    qosBreakdown: normalizeBreakdown(raw.qos_breakdown),
  };
}

export async function fetchMetrics(): Promise<DashboardMetrics> {
  const raw = await fetchJson<DashboardMetricsResponse>("/api/metrics");
  return normalizeMetrics(raw);
}

export async function fetchTopology(): Promise<TopologySnapshot> {
  return fetchJson<TopologySnapshot>("/api/topology");
}

export async function fetchFlow(): Promise<FlowGraph> {
  return fetchJson<FlowGraph>("/api/flow");
}

export function createEventStream(): EventSource {
  return new EventSource(buildUrl("/api/events/stream"));
}
