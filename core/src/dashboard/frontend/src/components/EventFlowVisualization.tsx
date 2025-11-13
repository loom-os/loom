import { useEffect, useState, useMemo } from "react";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

interface Event {
  id: string;
  type: string;
  topic: string;
  sender: string;
  threadId?: string;
  correlationId?: string;
  timestamp: number;
  qos?: "Realtime" | "Batched" | "Background";
}

interface EventFlowVisualizationProps {
  events: Event[];
  maxEvents?: number;
}

export const EventFlowVisualization = ({
  events,
  maxEvents = 50
}: EventFlowVisualizationProps) => {
  const [visibleEvents, setVisibleEvents] = useState<Event[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<string>("all");

  const agents = useMemo(() => {
    const agentSet = new Set(events.map(e => e.sender));
    return ["all", ...Array.from(agentSet).sort()];
  }, [events]);

  useEffect(() => {
    let filtered = events;
    if (selectedAgent !== "all") {
      filtered = events.filter(e => e.sender === selectedAgent);
    }
    setVisibleEvents(filtered.slice(-maxEvents));
  }, [events, maxEvents, selectedAgent]);

  const getQoSColor = (qos?: string) => {
    switch (qos) {
      case "Realtime":
        return "bg-primary/20 border-primary text-primary";
      case "Batched":
        return "bg-secondary/20 border-secondary text-secondary";
      case "Background":
        return "bg-accent/20 border-accent text-accent";
      default:
        return "bg-muted/20 border-muted text-muted-foreground";
    }
  };

  return (
    <Card className="p-6 bg-card/50 backdrop-blur border-border/50 shadow-card">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold text-foreground">Event Flow</h2>
        <div className="flex items-center gap-3">
          <Select value={selectedAgent} onValueChange={setSelectedAgent}>
            <SelectTrigger className="w-[140px] h-8">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {agents.map(agent => (
                <SelectItem key={agent} value={agent}>
                  {agent === "all" ? "All Agents" : agent}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Badge variant="outline" className="text-xs">
            {visibleEvents.length} events
          </Badge>
        </div>
      </div>

      <div className="space-y-2 max-h-[400px] overflow-y-auto custom-scrollbar">
        {visibleEvents.map((event) => (
          <div
            key={event.id}
            className="group relative flex items-center gap-3 p-3 rounded-lg bg-muted/30 hover:bg-muted/50 transition-colors duration-200 border border-border/30"
          >
            {/* Flow line indicator */}
            <div className="absolute left-0 top-0 bottom-0 w-1 bg-gradient-to-b from-primary via-secondary to-accent rounded-l-lg" />

            <div className="flex-1 min-w-0 ml-2">
              <div className="flex items-center gap-2 mb-1">
                <span className="text-sm font-mono text-foreground truncate">
                  {event.topic}
                </span>
                <Badge className={`text-xs ${getQoSColor(event.qos)}`}>
                  {event.qos ?? "Realtime"}
                </Badge>
              </div>

              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span className="font-mono">{event.type}</span>
                <span>•</span>
                <span>from {event.sender}</span>
                {event.threadId && (
                  <>
                    <span>•</span>
                    <span className="text-primary">thread: {event.threadId.slice(0, 8)}</span>
                  </>
                )}
              </div>
            </div>

            <div className="text-xs text-muted-foreground font-mono whitespace-nowrap">
              {new Date(event.timestamp).toLocaleTimeString()}
            </div>
          </div>
        ))}
      </div>

      <style>{`
        .custom-scrollbar::-webkit-scrollbar {
          width: 6px;
        }
        .custom-scrollbar::-webkit-scrollbar-track {
          background: hsl(var(--muted) / 0.3);
          border-radius: 3px;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb {
          background: hsl(var(--primary) / 0.5);
          border-radius: 3px;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb:hover {
          background: hsl(var(--primary) / 0.7);
        }
      `}</style>
    </Card>
  );
};
