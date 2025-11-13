import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Activity, Zap, Network, GitBranch } from "lucide-react";
import type { DashboardMetrics } from "@/lib/dashboardApi";

interface MetricsOverviewProps {
  metrics: DashboardMetrics;
}

export const MetricsOverview = ({ metrics }: MetricsOverviewProps) => {
  const metricCards = [
    {
      title: "Events/sec",
      value: metrics.eventsPerSecond.toFixed(1),
      icon: Activity,
      color: "text-primary",
      bgColor: "bg-primary/10",
      trend: "+12%",
    },
    {
      title: "Active Agents",
      value: metrics.activeAgents.toString(),
      icon: Network,
      color: "text-secondary",
      bgColor: "bg-secondary/10",
      trend: "stable",
    },
    {
      title: "Routing Decisions",
      value: metrics.routingDecisions.toString(),
      icon: GitBranch,
      color: "text-accent",
      bgColor: "bg-accent/10",
      trend: "+5%",
    },
    {
      title: "Avg Latency",
      value: `${metrics.averageLatencyMs}ms`,
      icon: Zap,
      color: "text-chart-4",
      bgColor: "bg-chart-4/10",
      trend: "-8%",
    },
  ];

  const totalQoS =
    metrics.qosBreakdown.realtime +
    metrics.qosBreakdown.batched +
    metrics.qosBreakdown.background;

  const qosPercentages = {
    realtime: totalQoS > 0 ? (metrics.qosBreakdown.realtime / totalQoS) * 100 : 0,
    batched: totalQoS > 0 ? (metrics.qosBreakdown.batched / totalQoS) * 100 : 0,
    background: totalQoS > 0 ? (metrics.qosBreakdown.background / totalQoS) * 100 : 0,
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
        {metricCards.map((metric) => (
          <Card
            key={metric.title}
            className="p-3 bg-card/50 backdrop-blur border-border/50 shadow-card hover:shadow-glow transition-all duration-300"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <p className="text-xs text-muted-foreground mb-1">
                  {metric.title}
                </p>
                <p className="text-2xl font-bold text-foreground mb-1.5">
                  {metric.value}
                </p>
                <Badge
                  variant="outline"
                  className="text-xs border-primary/30 text-primary"
                >
                  {metric.trend}
                </Badge>
              </div>
              <div className={`p-2 rounded-lg ${metric.bgColor}`}>
                <metric.icon className={`h-5 w-5 ${metric.color}`} />
              </div>
            </div>
          </Card>
        ))}
      </div>

      <Card className="p-4 bg-card/50 backdrop-blur border-border/50 shadow-card">
        <h3 className="text-base font-bold text-foreground mb-3">
          QoS Distribution
        </h3>
        <div className="space-y-2">
          <div>
            <div className="flex items-center justify-between mb-1">
              <span className="text-sm text-foreground">Realtime</span>
              <span className="text-sm font-mono text-primary">
                {qosPercentages.realtime.toFixed(1)}%
              </span>
            </div>
            <div className="h-1.5 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-primary to-primary/80 transition-all duration-500"
                style={{ width: `${qosPercentages.realtime}%` }}
              />
            </div>
          </div>

          <div>
            <div className="flex items-center justify-between mb-1">
              <span className="text-sm text-foreground">Batched</span>
              <span className="text-sm font-mono text-secondary">
                {qosPercentages.batched.toFixed(1)}%
              </span>
            </div>
            <div className="h-1.5 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-secondary to-secondary/80 transition-all duration-500"
                style={{ width: `${qosPercentages.batched}%` }}
              />
            </div>
          </div>

          <div>
            <div className="flex items-center justify-between mb-1">
              <span className="text-sm text-foreground">Background</span>
              <span className="text-sm font-mono text-accent">
                {qosPercentages.background.toFixed(1)}%
              </span>
            </div>
            <div className="h-1.5 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-accent to-accent/80 transition-all duration-500"
                style={{ width: `${qosPercentages.background}%` }}
              />
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
};
