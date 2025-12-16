import { useState, useEffect } from "react";
import { MetricsOverview } from "@/components/MetricsOverview";
import { EventFlowVisualization } from "@/components/EventFlowVisualization";
import { AgentNetworkGraph } from "@/components/AgentNetworkGraph";
import { AgentCommunication, type Communication } from "@/components/AgentCommunication";

const generateMockEvent = () => ({
  id: Math.random().toString(36).substr(2, 9),
  type: ["user.question", "research.request", "plan.execute", "llm.generate"][
    Math.floor(Math.random() * 4)
  ],
  topic: ["topic.plan", "topic.research", "topic.write", "topic.review"][
    Math.floor(Math.random() * 4)
  ],
  sender: ["planner", "researcher", "writer", "reviewer"][
    Math.floor(Math.random() * 4)
  ],
  threadId:
    Math.random() > 0.7 ? `thread-${Math.floor(Math.random() * 3)}` : undefined,
  correlationId:
    Math.random() > 0.5 ? `corr-${Math.floor(Math.random() * 5)}` : undefined,
  timestamp: Date.now(),
  qos: ["Realtime", "Batched", "Background"][Math.floor(Math.random() * 3)] as
    | "Realtime"
    | "Batched"
    | "Background",
});

const generateMockCommunication = (): Communication => {
  const types: Communication["type"][] = ["tool_call", "output", "message"];
  const agents = ["planner", "researcher", "writer", "reviewer"];
  const tools = ["web_search", "file_read", "llm_generate", "quality_check"];
  const type = types[Math.floor(Math.random() * types.length)];
  const agent = agents[Math.floor(Math.random() * agents.length)];

  const base = {
    id: Math.random().toString(36).substr(2, 9),
    timestamp: Date.now(),
    agent,
    type,
  };

  if (type === "tool_call") {
    const tool = tools[Math.floor(Math.random() * tools.length)];
    return {
      ...base,
      type: "tool_call",
      content: `Calling ${tool}`,
      tool,
      result: `Result: ${Math.random() > 0.5 ? "Success" : "Completed"} (${Math.floor(Math.random() * 500)}ms)`,
    };
  } else if (type === "output") {
    return {
      ...base,
      type: "output",
      content: [
        "Generated response",
        "Analysis complete",
        "Task finished",
        "Ready for review",
      ][Math.floor(Math.random() * 4)],
    };
  } else {
    const otherAgents = agents.filter((a) => a !== agent);
    return {
      ...base,
      type: "message",
      target: otherAgents[Math.floor(Math.random() * otherAgents.length)],
      content: [
        "Request processed",
        "Data ready",
        "Need input",
        "Awaiting approval",
      ][Math.floor(Math.random() * 4)],
    };
  }
};

const mockAgents = [
  {
    id: "planner",
    name: "Planner",
    topics: ["topic.plan"],
    capabilities: ["task.breakdown", "priority.assign"],
    status: "active" as const,
    connections: ["researcher", "writer"],
  },
  {
    id: "researcher",
    name: "Researcher",
    topics: ["topic.research"],
    capabilities: ["research.search", "data.analyze"],
    status: "processing" as const,
    connections: ["planner", "writer"],
  },
  {
    id: "writer",
    name: "Writer",
    topics: ["topic.write"],
    capabilities: ["content.generate", "style.adapt"],
    status: "active" as const,
    connections: ["planner", "researcher", "reviewer"],
  },
  {
    id: "reviewer",
    name: "Reviewer",
    topics: ["topic.review"],
    capabilities: ["quality.check", "feedback.generate"],
    status: "idle" as const,
    connections: ["writer"],
  },
];

const ObservabilityPage = () => {
  const [events, setEvents] = useState(() =>
    Array.from({ length: 10 }, generateMockEvent)
  );
  const [communications, setCommunications] = useState<Communication[]>(() =>
    Array.from({ length: 8 }, generateMockCommunication)
  );
  const [messages, setMessages] = useState<
    Array<{ from: string; to: string; timestamp: number }>
  >([]);

  const [metrics, setMetrics] = useState({
    eventsPerSecond: 12.5,
    activeAgents: 3,
    routingDecisions: 45,
    averageLatency: 28,
    qosBreakdown: {
      realtime: 60,
      batched: 30,
      background: 10,
    },
  });

  useEffect(() => {
    const eventInterval = setInterval(() => {
      const newEvent = generateMockEvent();
      setEvents((prev) => [...prev, newEvent].slice(-50));

      const possibleTargets =
        mockAgents.find((a) => a.id === newEvent.sender)?.connections || [];

      if (possibleTargets.length > 0 && Math.random() > 0.3) {
        setMessages((prev) =>
          [
            ...prev,
            {
              from: newEvent.sender,
              to: possibleTargets[
                Math.floor(Math.random() * possibleTargets.length)
              ],
              timestamp: Date.now(),
            },
          ].slice(-10)
        );
      }
    }, 2000);

    const commInterval = setInterval(() => {
      const newComm = generateMockCommunication();
      setCommunications((prev) => [...prev, newComm].slice(-30));

      if (newComm.type === "message" && newComm.target) {
        setMessages((prev) =>
          [
            ...prev,
            {
              from: newComm.agent,
              to: newComm.target!,
              timestamp: Date.now(),
            },
          ].slice(-10)
        );
      }
    }, 2500);

    const metricsInterval = setInterval(() => {
      setMetrics((prev) => ({
        ...prev,
        eventsPerSecond: 8 + Math.random() * 10,
        averageLatency: 20 + Math.floor(Math.random() * 20),
        qosBreakdown: {
          realtime: 50 + Math.floor(Math.random() * 20),
          batched: 25 + Math.floor(Math.random() * 15),
          background: 5 + Math.floor(Math.random() * 10),
        },
      }));
    }, 5000);

    return () => {
      clearInterval(eventInterval);
      clearInterval(commInterval);
      clearInterval(metricsInterval);
    };
  }, []);

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Observability Dashboard</h1>
      </div>

      <MetricsOverview metrics={metrics} />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <EventFlowVisualization events={events} />
        <AgentCommunication communications={communications} />
      </div>

      <AgentNetworkGraph agents={mockAgents} messages={messages} />
    </div>
  );
};

export default ObservabilityPage;
