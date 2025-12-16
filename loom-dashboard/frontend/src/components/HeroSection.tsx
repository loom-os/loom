import { Badge } from "@/components/ui/badge";
import { Sparkles } from "lucide-react";

export const HeroSection = () => {
  return (
    <div className="relative overflow-hidden rounded-2xl bg-gradient-mesh border border-border/50 shadow-glow p-8 mb-8">
      {/* Animated background grid */}
      <div className="absolute inset-0 opacity-20">
        <div className="absolute inset-0"
          style={{
            backgroundImage: `
              linear-gradient(to right, hsl(var(--primary) / 0.1) 1px, transparent 1px),
              linear-gradient(to bottom, hsl(var(--primary) / 0.1) 1px, transparent 1px)
            `,
            backgroundSize: '40px 40px'
          }}
        />
      </div>

      {/* Flowing accent line */}
      <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-transparent via-primary to-transparent animate-flow"
        style={{ backgroundSize: '200% 100%' }}
      />

      <div className="relative z-10">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-2 rounded-lg bg-primary/20 animate-pulse-glow">
            <Sparkles className="h-6 w-6 text-primary" />
          </div>
          <Badge variant="outline" className="text-xs border-primary/50 text-primary">
            Event-Driven AI OS
          </Badge>
        </div>

        <h1 className="text-4xl md:text-5xl font-bold text-foreground mb-3 bg-clip-text text-transparent bg-gradient-to-r from-foreground via-primary to-foreground">
          Loom Dashboard
        </h1>

        <p className="text-lg text-muted-foreground max-w-2xl">
          Real-time visualization of event-driven intelligence weaving through multi-agent collaboration
        </p>

        <div className="flex items-center gap-4 mt-6">
          <div className="flex items-center gap-2">
            <div className="h-2 w-2 rounded-full bg-primary animate-pulse" />
            <span className="text-sm text-muted-foreground">Live System</span>
          </div>
          <div className="h-4 w-px bg-border" />
          <div className="text-sm text-muted-foreground">
            Multi-Agent • Event Bus • Smart Routing
          </div>
        </div>
      </div>
    </div>
  );
};
