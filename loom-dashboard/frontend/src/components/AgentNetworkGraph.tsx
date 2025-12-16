import { useEffect, useRef } from "react";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

interface Agent {
  id: string;
  name: string;
  topics: string[];
  capabilities: string[];
  status: "active" | "idle" | "processing";
  connections: string[]; // Agent IDs this agent communicates with
}

interface Message {
  from: string;
  to: string;
  timestamp: number;
}

interface AgentNetworkGraphProps {
  agents: Agent[];
  messages?: Message[]; // Recent messages for animation
}

export const AgentNetworkGraph = ({ agents, messages = [] }: AgentNetworkGraphProps) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationFrameRef = useRef<number>();
  const particlesRef = useRef<Array<{
    from: number;
    to: number;
    progress: number;
    speed: number;
  }>>([]);

  useEffect(() => {
    // Add particles for new messages
    const now = Date.now();
    const recentMessages = messages.filter(m => now - m.timestamp < 3000);

    recentMessages.forEach(msg => {
      const fromIdx = agents.findIndex(a => a.id === msg.from);
      const toIdx = agents.findIndex(a => a.id === msg.to);
      if (fromIdx !== -1 && toIdx !== -1) {
        // Check if particle already exists for this connection
        const exists = particlesRef.current.some(
          p => p.from === fromIdx && p.to === toIdx && p.progress < 0.9
        );
        if (!exists) {
          particlesRef.current.push({
            from: fromIdx,
            to: toIdx,
            progress: 0,
            speed: 0.015 + Math.random() * 0.01,
          });
        }
      }
    });
  }, [messages, agents]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const width = rect.width;
    const height = rect.height;
    const centerX = width / 2;
    const centerY = height / 2;
    const radius = Math.min(width, height) * 0.35;

    // Position agents in a circle
    const positions = agents.map((_, i) => {
      const angle = (i / agents.length) * 2 * Math.PI - Math.PI / 2;
      return {
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle),
      };
    });

    const animate = () => {
      // Clear canvas
      ctx.clearRect(0, 0, width, height);

      // Draw connections
      agents.forEach((agent, i) => {
        agent.connections.forEach((connId) => {
          const targetIndex = agents.findIndex((a) => a.id === connId);
          if (targetIndex === -1) return;

          const start = positions[i];
          const end = positions[targetIndex];

          // Base connection line
          ctx.beginPath();
          ctx.moveTo(start.x, start.y);
          ctx.lineTo(end.x, end.y);
          ctx.strokeStyle = "rgba(6, 182, 212, 0.15)";
          ctx.lineWidth = 1.5;
          ctx.stroke();
        });
      });

      // Update and draw particles
      particlesRef.current = particlesRef.current.filter(particle => {
        particle.progress += particle.speed;

        if (particle.progress >= 1) {
          return false; // Remove completed particles
        }

        const start = positions[particle.from];
        const end = positions[particle.to];
        const x = start.x + (end.x - start.x) * particle.progress;
        const y = start.y + (end.y - start.y) * particle.progress;

        // Draw particle with glow
        const gradient = ctx.createRadialGradient(x, y, 0, x, y, 8);
        gradient.addColorStop(0, "rgba(6, 182, 212, 1)");
        gradient.addColorStop(0.5, "rgba(6, 182, 212, 0.6)");
        gradient.addColorStop(1, "rgba(6, 182, 212, 0)");

        ctx.beginPath();
        ctx.arc(x, y, 8, 0, 2 * Math.PI);
        ctx.fillStyle = gradient;
        ctx.fill();

        // Draw trail
        if (particle.progress > 0.1) {
          const trailLength = 0.15;
          const trailStart = Math.max(0, particle.progress - trailLength);
          const tx1 = start.x + (end.x - start.x) * trailStart;
          const ty1 = start.y + (end.y - start.y) * trailStart;

          const trailGradient = ctx.createLinearGradient(tx1, ty1, x, y);
          trailGradient.addColorStop(0, "rgba(6, 182, 212, 0)");
          trailGradient.addColorStop(1, "rgba(6, 182, 212, 0.5)");

          ctx.beginPath();
          ctx.moveTo(tx1, ty1);
          ctx.lineTo(x, y);
          ctx.strokeStyle = trailGradient;
          ctx.lineWidth = 3;
          ctx.stroke();
        }

        return true;
      });

    // Draw agent nodes
    agents.forEach((agent, i) => {
      const pos = positions[i];
      const nodeRadius = 30;

      // Status glow
      const glowColor =
        agent.status === "active"
          ? "rgba(6, 182, 212, 0.6)"
          : agent.status === "processing"
          ? "rgba(34, 197, 94, 0.6)"
          : "rgba(100, 116, 139, 0.4)";

      ctx.beginPath();
      ctx.arc(pos.x, pos.y, nodeRadius + 5, 0, 2 * Math.PI);
      ctx.fillStyle = glowColor;
      ctx.fill();

      // Node circle
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, nodeRadius, 0, 2 * Math.PI);
      ctx.fillStyle = "hsl(222 47% 8%)";
      ctx.fill();
      ctx.strokeStyle = "hsl(180 100% 50%)";
      ctx.lineWidth = 2;
      ctx.stroke();

      // Agent name
      ctx.fillStyle = "hsl(180 100% 95%)";
      ctx.font = "12px monospace";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(agent.name, pos.x, pos.y);
    });

      animationFrameRef.current = requestAnimationFrame(animate);
    };

    animate();

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [agents, messages]);

  const getStatusColor = (status: string) => {
    switch (status) {
      case "active":
        return "bg-primary text-primary-foreground";
      case "processing":
        return "bg-accent text-accent-foreground";
      case "idle":
        return "bg-muted text-muted-foreground";
      default:
        return "bg-muted text-muted-foreground";
    }
  };

  return (
    <Card className="p-6 bg-card/50 backdrop-blur border-border/50 shadow-card">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold text-foreground">Agent Network</h2>
        <Badge variant="outline" className="text-xs">
          {agents.length} agents
        </Badge>
      </div>

      <div className="relative">
        <canvas
          ref={canvasRef}
          className="w-full h-[400px] rounded-lg bg-background/50"
        />
      </div>

      <div className="mt-4 grid grid-cols-2 md:grid-cols-3 gap-2">
        {agents.map((agent) => (
          <div
            key={agent.id}
            className="p-2 rounded-lg bg-muted/30 border border-border/30 hover:bg-muted/50 transition-colors"
          >
            <div className="flex items-center justify-between mb-1">
              <span className="text-sm font-mono text-foreground">
                {agent.name}
              </span>
              <Badge className={`text-xs ${getStatusColor(agent.status)}`}>
                {agent.status}
              </Badge>
            </div>
            <div className="text-xs text-muted-foreground">
              {agent.capabilities.length} capabilities
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
};
