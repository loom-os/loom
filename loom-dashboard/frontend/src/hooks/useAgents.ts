/**
 * Agent Discovery Hook
 *
 * React hook for fetching and managing available agents from the Dashboard API.
 *
 * Features:
 * - Fetch list of registered agents
 * - Filter by status (active, idle, etc.)
 * - Automatic refresh on interval
 * - Type-safe agent information
 *
 * @example
 * ```tsx
 * const { agents, loading, error, refresh } = useAgents({
 *   refreshInterval: 5000,
 *   filterStatus: 'active',
 * });
 *
 * return (
 *   <div>
 *     {agents.map(agent => (
 *       <AgentCard key={agent.agent_id} agent={agent} />
 *     ))}
 *   </div>
 * );
 * ```
 */

import { useEffect, useState, useCallback } from 'react';

/**
 * Agent information from backend
 */
export interface Agent {
  agent_id: string;
  status: 'active' | 'idle' | 'inactive' | 'disconnected';
  subscribed_topics: string[];
  capabilities: string[];
  metadata: Record<string, string>;
  last_heartbeat?: number;
}

/**
 * API response format
 */
interface AgentsResponse {
  agents: Agent[];
  count: number;
  timestamp: string;
}

export interface UseAgentsOptions {
  /**
   * Auto-refresh interval in milliseconds
   * Set to 0 to disable auto-refresh
   * @default 10000 (10 seconds)
   */
  refreshInterval?: number;

  /**
   * Filter agents by status
   * @default undefined (no filter)
   */
  filterStatus?: Agent['status'];

  /**
   * Base URL for the API
   * @default 'http://localhost:3030'
   */
  baseUrl?: string;

  /**
   * Only return agents with specific topic patterns
   * Useful for filtering cognitive agents (e.g., topics containing "input")
   * @default undefined (no filter)
   */
  filterTopicPattern?: string;

  /**
   * Callback when agents are loaded
   */
  onLoad?: (agents: Agent[]) => void;

  /**
   * Callback when error occurs
   */
  onError?: (error: Error) => void;
}

export interface UseAgentsReturn {
  /**
   * List of available agents
   */
  agents: Agent[];

  /**
   * Whether data is being loaded
   */
  loading: boolean;

  /**
   * Error if fetch failed
   */
  error: Error | null;

  /**
   * Manually trigger a refresh
   */
  refresh: () => Promise<void>;

  /**
   * Get a specific agent by ID
   */
  getAgent: (agentId: string) => Agent | undefined;

  /**
   * Filter agents by capability
   */
  filterByCapability: (capability: string) => Agent[];

  /**
   * Get cognitive agents (heuristic: agents with "input" or "chat" topics)
   */
  getCognitiveAgents: () => Agent[];
}

/**
 * Hook for managing available agents
 */
export function useAgents(options: UseAgentsOptions = {}): UseAgentsReturn {
  const {
    refreshInterval = 10000,
    filterStatus,
    baseUrl = 'http://localhost:3030',
    filterTopicPattern,
    onLoad,
    onError,
  } = options;

  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchAgents = useCallback(async () => {
    try {
      const response = await fetch(`${baseUrl}/api/agents`);

      if (!response.ok) {
        throw new Error(`Failed to fetch agents: ${response.statusText}`);
      }

      const data: AgentsResponse = await response.json();

      let filteredAgents = data.agents;

      // Apply status filter
      if (filterStatus) {
        filteredAgents = filteredAgents.filter(
          (agent) => agent.status === filterStatus
        );
      }

      // Apply topic pattern filter
      if (filterTopicPattern) {
        filteredAgents = filteredAgents.filter((agent) =>
          agent.subscribed_topics.some((topic) =>
            topic.includes(filterTopicPattern)
          )
        );
      }

      setAgents(filteredAgents);
      setError(null);
      onLoad?.(filteredAgents);
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      setError(error);
      onError?.(error);
    } finally {
      setLoading(false);
    }
  }, [baseUrl, filterStatus, filterTopicPattern, onLoad, onError]);

  // Initial fetch and auto-refresh
  useEffect(() => {
    fetchAgents();

    if (refreshInterval > 0) {
      const interval = setInterval(fetchAgents, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [fetchAgents, refreshInterval]);

  const getAgent = useCallback(
    (agentId: string) => agents.find((agent) => agent.agent_id === agentId),
    [agents]
  );

  const filterByCapability = useCallback(
    (capability: string) =>
      agents.filter((agent) => agent.capabilities.includes(capability)),
    [agents]
  );

  const getCognitiveAgents = useCallback(() => {
    return agents.filter(
      (agent) =>
        agent.subscribed_topics.some(
          (topic) => topic.includes('input') || topic.includes('chat')
        ) || agent.metadata.type === 'cognitive'
    );
  }, [agents]);

  return {
    agents,
    loading,
    error,
    refresh: fetchAgents,
    getAgent,
    filterByCapability,
    getCognitiveAgents,
  };
}
