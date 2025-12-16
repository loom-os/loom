/**
 * Dashboard API Client
 *
 * Provides a high-level API for interacting with the Loom Dashboard backend.
 * Wraps WebSocket communication with type-safe interfaces.
 *
 * @example
 * ```tsx
 * const api = new DashboardAPI('ws://localhost:3030/ws');
 *
 * // Send a chat message
 * api.sendChatMessage('thread-123', 'Hello, world!');
 *
 * // Subscribe to chat responses
 * api.onChatChunk((chunk) => {
 *   console.log('Received:', chunk.content);
 * });
 * ```
 */

import type { WsMessage, ChatSettings, StreamStats } from '../hooks/useWebSocket';

export interface ChatMessageOptions {
  settings?: ChatSettings;
}

export interface ChatChunkCallback {
  (data: { thread_id: string; content: string; content_type: string; sequence: number }): void;
}

export interface ChatCompleteCallback {
  (data: { thread_id: string; stats: StreamStats }): void;
}

export interface EventStreamCallback {
  (data: {
    event_id: string;
    timestamp: string;
    topic: string;
    sender?: string;
    thread_id?: string;
    payload_preview: string;
  }): void;
}

export interface ErrorCallback {
  (data: { code: string; message: string; details?: unknown }): void;
}

/**
 * Dashboard API Client
 */
export class DashboardAPI {
  private ws: WebSocket | null = null;
  private listeners = new Map<string, Set<(data: unknown) => void>>();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;

  constructor(private url: string) {
    this.connect();
  }

  /**
   * Connect to WebSocket server
   */
  private connect(): void {
    try {
      this.ws = new WebSocket(this.url);

      this.ws.onopen = () => {
        console.log('[DashboardAPI] Connected');
        this.reconnectAttempts = 0;
        this.reconnectDelay = 1000;
      };

      this.ws.onclose = () => {
        console.log('[DashboardAPI] Disconnected');
        this.attemptReconnect();
      };

      this.ws.onerror = (error) => {
        console.error('[DashboardAPI] Error:', error);
      };

      this.ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data) as WsMessage;
          this.handleMessage(message);
        } catch (error) {
          console.error('[DashboardAPI] Failed to parse message:', error);
        }
      };
    } catch (error) {
      console.error('[DashboardAPI] Connection error:', error);
      this.attemptReconnect();
    }
  }

  /**
   * Attempt to reconnect with exponential backoff
   */
  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('[DashboardAPI] Max reconnection attempts reached');
      return;
    }

    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts);
    console.log(`[DashboardAPI] Reconnecting in ${delay}ms...`);

    setTimeout(() => {
      this.reconnectAttempts++;
      this.connect();
    }, delay);
  }

  /**
   * Handle incoming message
   */
  private handleMessage(message: WsMessage): void {
    const listeners = this.listeners.get(message.type);
    if (listeners) {
      listeners.forEach((callback) => callback(message));
    }
  }

  /**
   * Send a message to the server
   */
  private send(message: WsMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.warn('[DashboardAPI] Cannot send message: not connected');
    }
  }

  /**
   * Register a message listener
   */
  private on<T>(type: string, callback: (data: T) => void): () => void {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set());
    }
    this.listeners.get(type)!.add(callback as (data: unknown) => void);

    // Return unsubscribe function
    return () => {
      const listeners = this.listeners.get(type);
      if (listeners) {
        listeners.delete(callback as (data: unknown) => void);
      }
    };
  }

  // ===== Public API =====

  /**
   * Send a chat message
   */
  sendChatMessage(threadId: string, content: string, options?: ChatMessageOptions): void {
    this.send({
      type: 'chat_request',
      thread_id: threadId,
      content,
      settings: options?.settings,
    });
  }

  /**
   * Respond to a permission request
   */
  respondToPermission(requestId: string, approved: boolean, reason?: string): void {
    this.send({
      type: 'permission_response',
      request_id: requestId,
      approved,
      reason,
    });
  }

  /**
   * Subscribe to event topics
   */
  subscribe(topics: string[]): void {
    this.send({
      type: 'subscribe',
      topics,
    });
  }

  /**
   * Listen for chat chunk messages (streaming response)
   */
  onChatChunk(callback: ChatChunkCallback): () => void {
    return this.on('chat_chunk', callback);
  }

  /**
   * Listen for chat completion
   */
  onChatComplete(callback: ChatCompleteCallback): () => void {
    return this.on('chat_complete', callback);
  }

  /**
   * Listen for event stream messages
   */
  onEventStream(callback: EventStreamCallback): () => void {
    return this.on('event_stream', callback);
  }

  /**
   * Listen for error messages
   */
  onError(callback: ErrorCallback): () => void {
    return this.on('error', callback);
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }

  /**
   * Disconnect from server
   */
  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.listeners.clear();
  }
}

/**
 * Create a singleton API instance
 */
let apiInstance: DashboardAPI | null = null;

export function getAPI(url?: string): DashboardAPI {
  if (!apiInstance) {
    const wsUrl = url || `ws://${window.location.host}/ws`;
    apiInstance = new DashboardAPI(wsUrl);
  }
  return apiInstance;
}
