/**
 * Agent Selector Component
 *
 * Allows users to select which cognitive agent to chat with.
 * Displays agent status, capabilities, and last activity.
 *
 * Features:
 * - Visual agent cards with status indicators
 * - Capability badges
 * - Real-time status updates
 * - Responsive grid layout
 */

import { useEffect, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Bot, Circle, Clock, Zap, AlertCircle } from 'lucide-react';
import { useAgents, type Agent } from '@/hooks/useAgents';
import { cn } from '@/lib/utils';

interface AgentSelectorProps {
  /**
   * Currently selected agent ID
   */
  selectedAgentId?: string | null;

  /**
   * Callback when an agent is selected
   */
  onSelectAgent: (agentId: string) => void;

  /**
   * Optional filter: only show cognitive agents
   * @default true
   */
  cognitiveOnly?: boolean;

  /**
   * Optional CSS class name
   */
  className?: string;
}

/**
 * Format timestamp relative to now
 */
function formatLastSeen(timestamp?: number): string {
  if (!timestamp) return 'Never';

  const now = Date.now();
  const diff = now - timestamp;
  const seconds = Math.floor(diff / 1000);

  if (seconds < 60) return 'Just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

/**
 * Get status icon and color
 */
function getStatusDisplay(status: Agent['status']) {
  switch (status) {
    case 'active':
      return { icon: Circle, color: 'text-green-500', label: 'Active' };
    case 'idle':
      return { icon: Circle, color: 'text-yellow-500', label: 'Idle' };
    case 'inactive':
      return { icon: Circle, color: 'text-gray-400', label: 'Inactive' };
    case 'disconnected':
      return { icon: Circle, color: 'text-red-500', label: 'Disconnected' };
    default:
      return { icon: Circle, color: 'text-gray-400', label: 'Unknown' };
  }
}

/**
 * Agent card component
 */
function AgentCard({
  agent,
  isSelected,
  onSelect,
}: {
  agent: Agent;
  isSelected: boolean;
  onSelect: () => void;
}) {
  const statusDisplay = getStatusDisplay(agent.status);
  const StatusIcon = statusDisplay.icon;
  const isAvailable = agent.status === 'active' || agent.status === 'idle';

  return (
    <Card
      className={cn(
        'cursor-pointer transition-all hover:shadow-md',
        isSelected && 'ring-2 ring-primary ring-offset-2',
        !isAvailable && 'opacity-60'
      )}
      onClick={onSelect}
    >
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2">
            <div className="w-10 h-10 rounded-full bg-primary/10 flex items-center justify-center">
              <Bot className="h-5 w-5 text-primary" />
            </div>
            <div>
              <CardTitle className="text-base">{agent.agent_id}</CardTitle>
              <div className="flex items-center gap-1.5 mt-1">
                <StatusIcon className={cn('h-3 w-3 fill-current', statusDisplay.color)} />
                <span className="text-xs text-muted-foreground">{statusDisplay.label}</span>
              </div>
            </div>
          </div>
          {isSelected && (
            <Badge variant="default" className="bg-primary">
              Selected
            </Badge>
          )}
        </div>
      </CardHeader>

      <CardContent className="space-y-3">
        {/* Capabilities */}
        {agent.capabilities.length > 0 && (
          <div>
            <div className="flex items-center gap-1.5 mb-2">
              <Zap className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="text-xs font-medium text-muted-foreground">Capabilities</span>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {agent.capabilities.slice(0, 4).map((capability) => (
                <Badge key={capability} variant="secondary" className="text-xs">
                  {capability}
                </Badge>
              ))}
              {agent.capabilities.length > 4 && (
                <Badge variant="secondary" className="text-xs">
                  +{agent.capabilities.length - 4} more
                </Badge>
              )}
            </div>
          </div>
        )}

        {/* Topics */}
        {agent.subscribed_topics.length > 0 && (
          <div>
            <div className="flex items-center gap-1.5 mb-2">
              <Circle className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="text-xs font-medium text-muted-foreground">Topics</span>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {agent.subscribed_topics.slice(0, 2).map((topic) => (
                <Badge key={topic} variant="outline" className="text-xs font-mono">
                  {topic}
                </Badge>
              ))}
              {agent.subscribed_topics.length > 2 && (
                <Badge variant="outline" className="text-xs">
                  +{agent.subscribed_topics.length - 2}
                </Badge>
              )}
            </div>
          </div>
        )}

        {/* Last seen */}
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground pt-1">
          <Clock className="h-3.5 w-3.5" />
          <span>Last seen {formatLastSeen(agent.last_heartbeat)}</span>
        </div>
      </CardContent>
    </Card>
  );
}

/**
 * Agent Selector Component
 */
export default function AgentSelector({
  selectedAgentId,
  onSelectAgent,
  cognitiveOnly = true,
  className,
}: AgentSelectorProps) {
  const { agents, loading, error, getCognitiveAgents } = useAgents({
    refreshInterval: 10000, // Refresh every 10 seconds
    filterStatus: undefined, // Show all statuses
  });

  // Filter agents based on cognitiveOnly prop
  const displayAgents = cognitiveOnly ? getCognitiveAgents() : agents;

  // Sort: active first, then by agent_id
  const sortedAgents = [...displayAgents].sort((a, b) => {
    const statusPriority = { active: 0, idle: 1, inactive: 2, disconnected: 3 };
    const aPriority = statusPriority[a.status] ?? 4;
    const bPriority = statusPriority[b.status] ?? 4;

    if (aPriority !== bPriority) {
      return aPriority - bPriority;
    }
    return a.agent_id.localeCompare(b.agent_id);
  });

  // Auto-select first available agent if none selected
  useEffect(() => {
    if (!selectedAgentId && sortedAgents.length > 0) {
      const firstActive = sortedAgents.find(
        (agent) => agent.status === 'active' || agent.status === 'idle'
      );
      if (firstActive) {
        onSelectAgent(firstActive.agent_id);
      }
    }
  }, [selectedAgentId, sortedAgents, onSelectAgent]);

  if (loading) {
    return (
      <div className={cn('flex items-center justify-center p-8', className)}>
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-2" />
          <p className="text-sm text-muted-foreground">Loading agents...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={cn('flex items-center justify-center p-8', className)}>
        <div className="text-center">
          <AlertCircle className="h-8 w-8 text-destructive mx-auto mb-2" />
          <p className="text-sm text-muted-foreground">Failed to load agents</p>
          <p className="text-xs text-muted-foreground mt-1">{error.message}</p>
        </div>
      </div>
    );
  }

  if (sortedAgents.length === 0) {
    return (
      <div className={cn('flex items-center justify-center p-8', className)}>
        <div className="text-center">
          <Bot className="h-12 w-12 text-muted-foreground/50 mx-auto mb-3" />
          <p className="text-sm font-medium text-foreground">No agents available</p>
          <p className="text-xs text-muted-foreground mt-1">
            {cognitiveOnly
              ? 'No cognitive agents are currently registered'
              : 'No agents are currently registered'}
          </p>
          <p className="text-xs text-muted-foreground mt-2">
            Start an agent with <code className="bg-muted px-1.5 py-0.5 rounded">loom run</code>
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className={cn('space-y-4', className)}>
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">Select Agent</h3>
          <p className="text-sm text-muted-foreground">
            {sortedAgents.length} agent{sortedAgents.length !== 1 ? 's' : ''} available
          </p>
        </div>
      </div>

      <ScrollArea className="h-[calc(100vh-12rem)]">
        <div className="grid gap-3 pr-4">
          {sortedAgents.map((agent) => (
            <AgentCard
              key={agent.agent_id}
              agent={agent}
              isSelected={agent.agent_id === selectedAgentId}
              onSelect={() => onSelectAgent(agent.agent_id)}
            />
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
