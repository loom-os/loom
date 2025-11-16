import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Play, Pause, RefreshCw, Home } from "lucide-react";

interface SpanData {
  trace_id: string;
  span_id: string;
  parent_span_id?: string;
  name: string;
  start_time: number; // nanoseconds
  duration: number; // nanoseconds
  attributes: Record<string, string>;
  status: "ok" | "error" | "unset";
  error_message?: string;
}

interface TimelineTrack {
  id: string;
  label: string;
  spans: SpanData[];
}

const API_BASE = "";

async function fetchRecentSpans(limit: number = 100): Promise<SpanData[]> {
  const res = await fetch(`${API_BASE}/api/spans/recent?limit=${limit}`);
  if (!res.ok) throw new Error("Failed to fetch spans");
  return res.json();
}

function createSpanStream() {
  return new EventSource(`${API_BASE}/api/spans/stream`);
}

// Convert nanoseconds to milliseconds
const nsToMs = (ns: number) => ns / 1_000_000;

// Group spans by agent/component
function groupSpansByTrack(spans: SpanData[]): TimelineTrack[] {
  const tracks = new Map<string, SpanData[]>();

  for (const span of spans) {
    // Extract track label from span name or attributes
    let trackId = "Unknown";

    if (span.attributes.agent_id) {
      trackId = span.attributes.agent_id;
    } else if (span.name.startsWith("bridge.")) {
      trackId = "Bridge";
    } else if (span.name.startsWith("event_bus.")) {
      trackId = "EventBus";
    } else if (span.name.startsWith("action_broker.")) {
      trackId = "ActionBroker";
    } else if (span.name.includes("agent.")) {
      // Extract agent name from span name
      const match = span.name.match(/agent\.(\w+)/);
      if (match) trackId = match[1];
    }

    if (!tracks.has(trackId)) {
      tracks.set(trackId, []);
    }
    tracks.get(trackId)!.push(span);
  }

  return Array.from(tracks.entries()).map(([id, spans]) => ({
    id,
    label: id,
    spans: spans.sort((a, b) => a.start_time - b.start_time),
  }));
}

const Timeline = () => {
  const [spans, setSpans] = useState<SpanData[]>([]);
  const [isLive, setIsLive] = useState(true);
  const [selectedTrace, setSelectedTrace] = useState<string>("all");
  const [maxSpans, setMaxSpans] = useState(200);

  // Fetch initial spans
  const { data: initialSpans, refetch } = useQuery({
    queryKey: ["spans", maxSpans],
    queryFn: () => fetchRecentSpans(maxSpans),
    refetchInterval: isLive ? false : 5000,
  });

  // Update spans when initial data loads
  useEffect(() => {
    if (initialSpans) {
      setSpans(initialSpans);
    }
  }, [initialSpans]);

  // Live SSE stream
  useEffect(() => {
    if (!isLive) return;

    let source = createSpanStream();
    let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

    const handleMessage = (event: MessageEvent<string>) => {
      if (!event.data) return;

      try {
        const newSpans = JSON.parse(event.data) as SpanData[];
        setSpans((prev) => [...prev, ...newSpans].slice(-maxSpans));
      } catch (error) {
        console.warn("Failed to parse span event", error);
      }
    };

    const scheduleReconnect = () => {
      reconnectTimer = setTimeout(() => {
        source.close();
        source = createSpanStream();
        source.addEventListener("spans", handleMessage);
        source.onerror = scheduleReconnect;
      }, 2000);
    };

    source.addEventListener("spans", handleMessage);
    source.onerror = scheduleReconnect;

    return () => {
      source.close();
      if (reconnectTimer) clearTimeout(reconnectTimer);
    };
  }, [isLive, maxSpans]);

  // Filter and organize spans
  const { tracks, traces, timeRange } = useMemo(() => {
    const filtered =
      selectedTrace === "all"
        ? spans
        : spans.filter((s) => s.trace_id === selectedTrace);

    const uniqueTraces = Array.from(new Set(spans.map((s) => s.trace_id)));

    // Calculate time range
    if (filtered.length === 0) {
      return { tracks: [], traces: uniqueTraces, timeRange: { min: 0, max: 0 } };
    }

    const times = filtered.map((s) => s.start_time);
    const min = Math.min(...times);
    const max = Math.max(...times.map((t, i) => t + filtered[i].duration));

    return {
      tracks: groupSpansByTrack(filtered),
      traces: uniqueTraces,
      timeRange: { min, max },
    };
  }, [spans, selectedTrace]);

  const totalDurationMs = nsToMs(timeRange.max - timeRange.min);

  const getStatusColor = (status: string) => {
    switch (status) {
      case "ok":
        return "bg-accent/30 border-accent hover:bg-accent/50";
      case "error":
        return "bg-destructive/30 border-destructive hover:bg-destructive/50";
      default:
        return "bg-muted/30 border-muted-foreground hover:bg-muted/50";
    }
  };

  return (
    <div className="min-h-screen bg-background">
      <div className="container mx-auto px-4 py-8">
        {/* Header */}
        <div className="mb-8">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h1 className="text-4xl font-bold bg-gradient-to-r from-primary via-secondary to-accent bg-clip-text text-transparent mb-2">
                Trace Timeline
              </h1>
              <p className="text-muted-foreground">
                Visualize distributed traces across agents and components
              </p>
            </div>
            <Link to="/">
              <Button variant="outline" size="sm" className="gap-2">
                <Home className="h-4 w-4" />
                Back to Dashboard
              </Button>
            </Link>
          </div>
        </div>

        {/* Controls */}
        <Card className="p-4 mb-6 bg-card/50 backdrop-blur border-border/50">
          <div className="flex items-center justify-between flex-wrap gap-4">
            <div className="flex items-center gap-3">
              <Button
                variant={isLive ? "default" : "outline"}
                size="sm"
                onClick={() => setIsLive(!isLive)}
                className="gap-2"
              >
                {isLive ? (
                  <>
                    <Pause className="h-4 w-4" />
                    Pause
                  </>
                ) : (
                  <>
                    <Play className="h-4 w-4" />
                    Resume
                  </>
                )}
              </Button>

              <Button
                variant="outline"
                size="sm"
                onClick={() => refetch()}
                className="gap-2"
              >
                <RefreshCw className="h-4 w-4" />
                Refresh
              </Button>

              <Select value={selectedTrace} onValueChange={setSelectedTrace}>
                <SelectTrigger className="w-[200px] h-9">
                  <SelectValue placeholder="Select trace" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Traces</SelectItem>
                  {traces.slice(0, 20).map((traceId) => (
                    <SelectItem key={traceId} value={traceId}>
                      {traceId.slice(0, 16)}...
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="flex items-center gap-3">
              <Badge variant="outline" className="text-xs">
                {spans.length} spans
              </Badge>
              <Badge variant="outline" className="text-xs">
                {tracks.length} tracks
              </Badge>
              <Badge variant="outline" className="text-xs">
                {totalDurationMs.toFixed(1)}ms total
              </Badge>
            </div>
          </div>
        </Card>

        {/* Timeline Visualization */}
        <Card className="p-6 bg-card/50 backdrop-blur border-border/50 shadow-card">
          {tracks.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <p className="text-lg mb-2">No spans yet</p>
              <p className="text-sm">
                Waiting for trace data from agents...
              </p>
            </div>
          ) : (
            <div className="space-y-4">
              {tracks.map((track) => (
                <div key={track.id} className="relative">
                  {/* Track Label */}
                  <div className="flex items-center gap-3 mb-2">
                    <div className="w-32 flex-shrink-0">
                      <span className="text-sm font-mono text-foreground">
                        {track.label}
                      </span>
                    </div>

                    {/* Timeline Bar */}
                    <div className="flex-1 relative h-12 bg-muted/20 rounded-lg border border-border/30 overflow-hidden">
                      {/* Time grid lines */}
                      <div className="absolute inset-0 flex">
                        {[0, 0.25, 0.5, 0.75, 1].map((ratio) => (
                          <div
                            key={ratio}
                            className="absolute top-0 bottom-0 w-px bg-border/20"
                            style={{ left: `${ratio * 100}%` }}
                          />
                        ))}
                      </div>

                      {/* Spans */}
                      {track.spans.map((span) => {
                        const startRatio =
                          (span.start_time - timeRange.min) /
                          (timeRange.max - timeRange.min);
                        const durationRatio =
                          span.duration / (timeRange.max - timeRange.min);

                        const left = `${startRatio * 100}%`;
                        const width = `${Math.max(durationRatio * 100, 0.5)}%`;

                        return (
                          <div
                            key={span.span_id}
                            className={`absolute top-1/2 -translate-y-1/2 h-8 rounded border transition-all duration-200 cursor-pointer group ${getStatusColor(
                              span.status,
                            )}`}
                            style={{ left, width }}
                            title={`${span.name}\nDuration: ${nsToMs(span.duration).toFixed(2)}ms\nStatus: ${span.status}`}
                          >
                            {/* Span label (only show if wide enough) */}
                            {durationRatio > 0.05 && (
                              <div className="absolute inset-0 flex items-center px-2">
                                <span className="text-xs font-mono truncate text-foreground">
                                  {span.name}
                                </span>
                              </div>
                            )}

                            {/* Hover tooltip */}
                            <div className="invisible group-hover:visible absolute top-full left-0 mt-1 z-10 w-64 p-2 bg-popover border border-border rounded-lg shadow-lg text-xs">
                              <div className="space-y-1">
                                <div className="font-semibold text-foreground">
                                  {span.name}
                                </div>
                                <div className="text-muted-foreground">
                                  Duration: {nsToMs(span.duration).toFixed(2)}ms
                                </div>
                                <div className="text-muted-foreground">
                                  Status: {span.status}
                                </div>
                                {span.attributes.topic && (
                                  <div className="text-muted-foreground">
                                    Topic: {span.attributes.topic}
                                  </div>
                                )}
                                {span.error_message && (
                                  <div className="text-destructive mt-1">
                                    Error: {span.error_message}
                                  </div>
                                )}
                              </div>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                </div>
              ))}

              {/* Time axis */}
              <div className="flex items-center gap-3 mt-6 pt-4 border-t border-border/30">
                <div className="w-32 flex-shrink-0"></div>
                <div className="flex-1 flex justify-between text-xs text-muted-foreground font-mono">
                  <span>0ms</span>
                  <span>{(totalDurationMs * 0.25).toFixed(1)}ms</span>
                  <span>{(totalDurationMs * 0.5).toFixed(1)}ms</span>
                  <span>{(totalDurationMs * 0.75).toFixed(1)}ms</span>
                  <span>{totalDurationMs.toFixed(1)}ms</span>
                </div>
              </div>
            </div>
          )}
        </Card>

        {/* Footer */}
        <div className="mt-8 text-center text-sm text-muted-foreground">
          <p>
            Hover over spans for details • Select a trace to filter • Pause to
            examine
          </p>
        </div>
      </div>
    </div>
  );
};

export default Timeline;
