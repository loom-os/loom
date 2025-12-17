import { useState, useEffect } from "react";
import { MetricsOverview } from "@/components/MetricsOverview";
import { EventFlowVisualization } from "@/components/EventFlowVisualization";
import { AgentNetworkGraph } from "@/components/AgentNetworkGraph";
import { AgentCommunication } from "@/components/AgentCommunication";
import { useObservability } from "@/hooks/useObservability";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";

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
  const { events, communications, metrics, isConnected, isConnecting } =
    useObservability();

  const [messages, setMessages] = useState<
    Array<{ from: string; to: string; timestamp: number }>
  >([]);

  // Extract agent messages from communications
  useEffect(() => {
    const recentMessages = communications
      .filter((c) => c.type === "message" && c.target)
      .map((c) => ({
        from: c.agent,
        to: c.target!,
        timestamp: c.timestamp,
      }))
      .slice(-10);

    setMessages(recentMessages);
  }, [communications]);

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Observability Dashboard</h1>
        <div className="flex items-center gap-2">
          {isConnecting && (
            <Badge variant="outline" className="border-yellow-500 text-yellow-500">
              Connecting...
            </Badge>
          )}
          {!isConnecting && isConnected && (
            <Badge variant="outline" className="border-green-500 text-green-500">
              ● Live
            </Badge>
          )}
          {!isConnecting && !isConnected && (
            <Badge variant="outline" className="border-red-500 text-red-500">
              ● Disconnected
            </Badge>
          )}
        </div>
      </div>

      {!isConnected && !isConnecting && (
        <Alert>
          <AlertDescription>
            Unable to connect to the Loom Dashboard backend. Make sure the
            server is running on <code>ws://localhost:3030/ws</code>.
          </AlertDescription>
        </Alert>
      )}

      {events.length === 0 && isConnected && (
        <Alert>
          <AlertDescription>
            Connected and waiting for events. Events will appear here as they
            flow through the EventBus.
          </AlertDescription>
        </Alert>
      )}

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
