/**
 * WebSocket Hook
 *
 * React hook for managing WebSocket connections to the Loom Dashboard backend.
 *
 * Features:
 * - Automatic connection management
 * - Reconnection with exponential backoff
 * - Message sending/receiving
 * - Connection state tracking
 * - Type-safe message handling
 *
 * @example
 * ```tsx
 * const {
 *   isConnected,
 *   send,
 *   lastMessage,
 *   error
 * } = useWebSocket('ws://localhost:3030/ws');
 *
 * // Send a message
 * send({
 *   type: 'chat_request',
 *   thread_id: 'thread-123',
 *   content: 'Hello, world!'
 * });
 * ```
 */

import { useEffect, useRef, useState, useCallback } from 'react';

/**
 * WebSocket message types matching backend protocol
 */
export type WsMessage =
  // Client → Server
  | {
      type: 'chat_request';
      thread_id: string;
      content: string;
      settings?: ChatSettings;
    }
  | {
      type: 'permission_response';
      request_id: string;
      approved: boolean;
      reason?: string;
    }
  | {
      type: 'subscribe';
      topics: string[];
    }
  // Server → Client
  | {
      type: 'chat_chunk';
      thread_id: string;
      content: string;
      content_type: string;
      sequence: number;
    }
  | {
      type: 'chat_complete';
      thread_id: string;
      stats: StreamStats;
    }
  | {
      type: 'event_stream';
      event_id: string;
      timestamp: string;
      topic: string;
      sender?: string;
      thread_id?: string;
      payload_preview: string;
    }
  | {
      type: 'topology_update';
      agents: AgentInfo[];
      connections: AgentConnection[];
    }
  | {
      type: 'metrics_update';
      timestamp: string;
      metrics: SystemMetrics;
    }
  | {
      type: 'permission_request';
      request_id: string;
      action: string;
      description: string;
      auto_approve_after?: number;
    }
  | {
      type: 'error';
      code: string;
      message: string;
      details?: unknown;
    };

export interface ChatSettings {
  model?: string;
  temperature?: number;
  max_tokens?: number;
}

export interface StreamStats {
  duration_ms: number;
  total_tokens: number;
  tool_calls: number;
}

export interface AgentInfo {
  id: string;
  name: string;
  status: string;
  capabilities: string[];
}

export interface AgentConnection {
  source: string;
  target: string;
  type: string;
}

export interface SystemMetrics {
  cpu_usage: number;
  memory_usage: number;
  active_connections: number;
  messages_per_second: number;
}

export interface UseWebSocketOptions {
  /**
   * Automatically reconnect on disconnect
   * @default true
   */
  reconnect?: boolean;

  /**
   * Maximum number of reconnection attempts
   * @default 5
   */
  maxReconnectAttempts?: number;

  /**
   * Initial reconnection delay in milliseconds
   * @default 1000
   */
  reconnectDelay?: number;

  /**
   * Callback when connection opens
   */
  onOpen?: () => void;

  /**
   * Callback when connection closes
   */
  onClose?: () => void;

  /**
   * Callback when message is received
   */
  onMessage?: (message: WsMessage) => void;

  /**
   * Callback when error occurs
   */
  onError?: (error: Event) => void;
}

export interface UseWebSocketReturn {
  /**
   * Whether the WebSocket is currently connected
   */
  isConnected: boolean;

  /**
   * Whether the WebSocket is attempting to connect
   */
  isConnecting: boolean;

  /**
   * Last received message
   */
  lastMessage: WsMessage | null;

  /**
   * Last error that occurred
   */
  error: Event | null;

  /**
   * Send a message to the server
   */
  send: (message: WsMessage) => void;

  /**
   * Manually reconnect
   */
  reconnect: () => void;

  /**
   * Manually disconnect
   */
  disconnect: () => void;
}

/**
 * Hook for managing WebSocket connections
 */
export function useWebSocket(
  url: string,
  options: UseWebSocketOptions = {}
): UseWebSocketReturn {
  const {
    reconnect: shouldReconnect = true,
    maxReconnectAttempts = 5,
    reconnectDelay: initialReconnectDelay = 1000,
    onOpen,
    onClose,
    onMessage,
    onError,
  } = options;

  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [lastMessage, setLastMessage] = useState<WsMessage | null>(null);
  const [error, setError] = useState<Event | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout>();
  const reconnectAttemptsRef = useRef(0);
  const reconnectDelayRef = useRef(initialReconnectDelay);
  const shouldConnectRef = useRef(true);

  const connect = useCallback(() => {
    if (!shouldConnectRef.current) {
      return;
    }

    setIsConnecting(true);
    setError(null);

    try {
      const ws = new WebSocket(url);

      ws.onopen = () => {
        console.log('[WebSocket] Connected');
        setIsConnected(true);
        setIsConnecting(false);
        setError(null);
        reconnectAttemptsRef.current = 0;
        reconnectDelayRef.current = initialReconnectDelay;
        onOpen?.();
      };

      ws.onclose = () => {
        console.log('[WebSocket] Disconnected');
        setIsConnected(false);
        setIsConnecting(false);
        onClose?.();

        // Attempt reconnection
        if (
          shouldReconnect &&
          shouldConnectRef.current &&
          reconnectAttemptsRef.current < maxReconnectAttempts
        ) {
          const delay = reconnectDelayRef.current;
          console.log(`[WebSocket] Reconnecting in ${delay}ms...`);

          reconnectTimeoutRef.current = setTimeout(() => {
            reconnectAttemptsRef.current += 1;
            reconnectDelayRef.current = Math.min(delay * 2, 30000); // Max 30s
            connect();
          }, delay);
        }
      };

      ws.onerror = (event) => {
        console.error('[WebSocket] Error:', event);
        setError(event);
        setIsConnecting(false);
        onError?.(event);
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data) as WsMessage;
          setLastMessage(message);
          onMessage?.(message);
        } catch (err) {
          console.error('[WebSocket] Failed to parse message:', err);
        }
      };

      wsRef.current = ws;
    } catch (err) {
      console.error('[WebSocket] Connection error:', err);
      setIsConnecting(false);
      setError(err as Event);
    }
  }, [url, shouldReconnect, maxReconnectAttempts, initialReconnectDelay, onOpen, onClose, onMessage, onError]);

  const disconnect = useCallback(() => {
    shouldConnectRef.current = false;
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
    }
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    setIsConnected(false);
    setIsConnecting(false);
  }, []);

  const reconnectManually = useCallback(() => {
    disconnect();
    shouldConnectRef.current = true;
    reconnectAttemptsRef.current = 0;
    reconnectDelayRef.current = initialReconnectDelay;
    connect();
  }, [disconnect, connect, initialReconnectDelay]);

  const send = useCallback((message: WsMessage) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      try {
        wsRef.current.send(JSON.stringify(message));
      } catch (err) {
        console.error('[WebSocket] Failed to send message:', err);
      }
    } else {
      console.warn('[WebSocket] Cannot send message: not connected');
    }
  }, []);

  // Connect on mount
  useEffect(() => {
    shouldConnectRef.current = true;
    connect();

    return () => {
      shouldConnectRef.current = false;
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [url]); // Only reconnect when URL changes

  return {
    isConnected,
    isConnecting,
    lastMessage,
    error,
    send,
    reconnect: reconnectManually,
    disconnect,
  };
}
