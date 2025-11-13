import { useEffect, useState } from "react";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

export interface Communication {
  id: string;
  timestamp: number;
  agent: string;
  type: "tool_call" | "output" | "message";
  target?: string; // For messages between agents
  content: string;
  tool?: string; // For tool calls
  result?: string; // For tool results
  threadId?: string;
}

interface AgentCommunicationProps {
  communications: Communication[];
  maxItems?: number;
}

export const AgentCommunication = ({
  communications,
  maxItems = 30
}: AgentCommunicationProps) => {
  const [visible, setVisible] = useState<Communication[]>([]);

  useEffect(() => {
    setVisible(communications.slice(-maxItems));
  }, [communications, maxItems]);

  const getTypeColor = (type: string) => {
    switch (type) {
      case "tool_call":
        return "bg-accent/20 border-accent text-accent";
      case "output":
        return "bg-primary/20 border-primary text-primary";
      case "message":
        return "bg-secondary/20 border-secondary text-secondary";
      default:
        return "bg-muted/20 border-muted text-muted-foreground";
    }
  };

  const getTypeLabel = (type: string) => {
    switch (type) {
      case "tool_call":
        return "Tool";
      case "output":
        return "Output";
      case "message":
        return "Message";
      default:
        return type;
    }
  };

  return (
    <Card className="p-6 bg-card/50 backdrop-blur border-border/50 shadow-card">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold text-foreground">Agent Communications</h2>
        <Badge variant="outline" className="text-xs">
          {visible.length} actions
        </Badge>
      </div>

      <div className="space-y-2 max-h-[400px] overflow-y-auto custom-scrollbar">
        {visible.map((comm) => (
          <div
            key={comm.id}
            className="relative p-3 rounded-lg bg-muted/30 hover:bg-muted/50 transition-colors duration-200 border border-border/30"
          >
            <div className="flex items-start gap-3">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-sm font-mono text-foreground font-semibold">
                    {comm.agent}
                  </span>
                  <Badge className={`text-xs ${getTypeColor(comm.type)}`}>
                    {getTypeLabel(comm.type)}
                  </Badge>
                  {comm.tool && (
                    <span className="text-xs text-muted-foreground font-mono">
                      → {comm.tool}
                    </span>
                  )}
                  {comm.target && (
                    <span className="text-xs text-muted-foreground">
                      → {comm.target}
                    </span>
                  )}
                </div>

                <div className="text-sm text-foreground/90 mb-1">
                  {comm.content}
                </div>

                {comm.threadId && (
                  <div className="text-xs text-muted-foreground font-mono">
                    thread: {comm.threadId}
                  </div>
                )}

                {comm.result && (
                  <div className="text-xs text-muted-foreground font-mono mt-1 pl-2 border-l-2 border-primary/30">
                    {comm.result}
                  </div>
                )}
              </div>

              <div className="text-xs text-muted-foreground font-mono whitespace-nowrap">
                {new Date(comm.timestamp).toLocaleTimeString()}
              </div>
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
