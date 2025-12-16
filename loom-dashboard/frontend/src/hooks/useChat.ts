import { useState, useCallback, useRef, useEffect } from 'react';
import { useWebSocket, type WsMessage } from './useWebSocket';

export type ContentType = 'text' | 'thinking' | 'tool_call' | 'tool_result' | 'error';

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: Date;
  agentId?: string;
  agentName?: string;
  contentType?: ContentType;
  isStreaming?: boolean;
  toolCall?: {
    name: string;
    args: Record<string, any>;
  };
  toolResult?: {
    success: boolean;
    result: string;
  };
  metadata?: Record<string, any>;
}

export interface ChatSettings {
  model?: string;
  temperature?: number;
  maxTokens?: number;
  systemPrompt?: string;
}

interface UseChatOptions {
  wsUrl?: string;
  onError?: (error: string) => void;
  onAgentSwitch?: (fromAgent: string | null, toAgent: string) => void;
}

export interface UseChatReturn {
  messages: ChatMessage[];
  sendMessage: (content: string, agentId: string, settings?: ChatSettings) => void;
  clearMessages: () => void;
  isConnected: boolean;
  isLoading: boolean;
  threadId: string;
  selectedAgentId: string | null;
  setSelectedAgentId: (agentId: string | null) => void;
  startNewThread: () => void;
  usedAgents: string[];
}

/**
 * Hook for managing chat state and WebSocket communication
 *
 * Features:
 * - Multi-agent conversation management
 * - Streaming message handling with content type detection
 * - Automatic message accumulation
 * - Thread management
 *
 * @example
 * ```tsx
 * const chat = useChat({
 *   onAgentSwitch: (from, to) => console.log(`Switched from ${from} to ${to}`)
 * });
 *
 * // Send message
 * chat.sendMessage("Hello", "agent-1");
 *
 * // Render messages
 * chat.messages.map(msg => <Message key={msg.id} {...msg} />)
 * ```
 */
export function useChat(options: UseChatOptions = {}): UseChatReturn {
  const {
    wsUrl = `ws://${window.location.host}/ws`,
    onError,
    onAgentSwitch
  } = options;

  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [threadId, setThreadId] = useState<string>(`thread-${Date.now()}`);

  // Refs for streaming accumulation
  const streamingMessageRef = useRef<{ id: string; content: string; contentType: ContentType } | null>(null);
  const previousAgentRef = useRef<string | null>(null);

  // WebSocket connection
  const { isConnected, send, lastMessage } = useWebSocket(wsUrl, {
    reconnect: true,
    onOpen: () => {
      console.log('[useChat] WebSocket connected');
    },
    onClose: () => {
      console.log('[useChat] WebSocket disconnected');
    },
  });

  // Parse content type from metadata
  const parseContentType = useCallback((metadata?: Record<string, any>): ContentType => {
    if (!metadata) return 'text';

    const contentType = metadata['stream.content_type'] || metadata['content_type'];
    if (contentType === 'thinking') return 'thinking';
    if (contentType === 'tool_call') return 'tool_call';
    if (contentType === 'tool_result') return 'tool_result';
    if (contentType === 'error') return 'error';

    return 'text';
  }, []);

  // Parse tool call from content
  const parseToolCall = useCallback((content: string): ChatMessage['toolCall'] | undefined => {
    try {
      // Try to parse JSON tool call format
      // Example: {"tool": "web:search", "args": {"query": "weather"}}
      const match = content.match(/\{[\s\S]*?"tool"[\s\S]*?\}/);
      if (match) {
        const parsed = JSON.parse(match[0]);
        return {
          name: parsed.tool || parsed.name || parsed.action,
          args: parsed.args || parsed.arguments || parsed.input || {}
        };
      }
    } catch (e) {
      // Not a JSON tool call
    }
    return undefined;
  }, []);

  // Handle incoming WebSocket messages
  useEffect(() => {
    if (!lastMessage) return;

    switch (lastMessage.type) {
      case 'chat_chunk': {
        const { thread_id, content, sequence, content_type } = lastMessage;
        if (thread_id !== threadId) return;

        const parsedContentType = parseContentType({ 'stream.content_type': content_type });

        // Create or update streaming message
        if (sequence === 0 || !streamingMessageRef.current) {
          // First chunk - create new message
          const messageId = `msg-${Date.now()}`;
          streamingMessageRef.current = {
            id: messageId,
            content,
            contentType: parsedContentType
          };

          const newMessage: ChatMessage = {
            id: messageId,
            role: 'assistant',
            content,
            timestamp: new Date(),
            agentId: selectedAgentId || undefined,
            agentName: selectedAgentId || undefined,
            contentType: parsedContentType,
            isStreaming: true,
            toolCall: parsedContentType === 'tool_call' ? parseToolCall(content) : undefined,
          };

          setMessages((prev) => [...prev, newMessage]);
        } else {
          // Subsequent chunks - append content
          streamingMessageRef.current.content += content;
          const messageId = streamingMessageRef.current.id;
          const fullContent = streamingMessageRef.current.content;

          setMessages((prev) =>
            prev.map((msg) =>
              msg.id === messageId
                ? {
                    ...msg,
                    content: fullContent,
                    isStreaming: true,
                    toolCall: msg.contentType === 'tool_call' ? parseToolCall(fullContent) : msg.toolCall,
                  }
                : msg
            )
          );
        }
        break;
      }

      case 'chat_complete': {
        const { thread_id, stats } = lastMessage;
        if (thread_id !== threadId) return;

        // Mark message as completed (no longer streaming)
        if (streamingMessageRef.current) {
          const messageId = streamingMessageRef.current.id;
          setMessages((prev) =>
            prev.map((msg) =>
              msg.id === messageId
                ? {
                    ...msg,
                    isStreaming: false,
                    metadata: stats
                  }
                : msg
            )
          );
        }

        // Finish streaming
        streamingMessageRef.current = null;
        setIsLoading(false);
        break;
      }

      case 'error': {
        console.error('[useChat] Error:', lastMessage.message);
        setIsLoading(false);

        // Show error message
        const errorMessage: ChatMessage = {
          id: `error-${Date.now()}`,
          role: 'assistant',
          content: lastMessage.message || 'An error occurred',
          contentType: 'error',
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, errorMessage]);

        onError?.(lastMessage.message || 'An error occurred');
        break;
      }

      default:
        // Handle other message types if needed
        break;
    }
  }, [lastMessage, threadId, selectedAgentId, parseContentType, parseToolCall, onError]);

  // Send message
  const sendMessage = useCallback((
    content: string,
    agentId: string,
    settings?: ChatSettings
  ) => {
    if (!isConnected) {
      console.warn('[useChat] Cannot send message: not connected');
      onError?.('Not connected to server');
      return;
    }

    if (!agentId) {
      console.warn('[useChat] Cannot send message: no agent specified');
      onError?.('No agent selected');
      return;
    }

    // Track agent switch
    if (previousAgentRef.current !== agentId) {
      onAgentSwitch?.(previousAgentRef.current, agentId);

      // Add system message for agent switch
      if (previousAgentRef.current && messages.length > 0) {
        const systemMessage: ChatMessage = {
          id: `system-${Date.now()}`,
          role: 'system',
          content: `🔄 Switched from ${previousAgentRef.current} to ${agentId}`,
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, systemMessage]);
      }

      previousAgentRef.current = agentId;
    }

    // Add user message
    const userMessage: ChatMessage = {
      id: `user-${Date.now()}`,
      role: 'user',
      content,
      timestamp: new Date(),
      agentId,
    };
    setMessages((prev) => [...prev, userMessage]);
    setIsLoading(true);

    // Send to WebSocket
    send({
      type: 'chat_request',
      thread_id: threadId,
      content,
      agent_id: agentId,
      settings: settings ? {
        model: settings.model,
        temperature: settings.temperature,
        max_tokens: settings.maxTokens,
      } : undefined,
    } as any);
  }, [isConnected, threadId, messages.length, send, onError, onAgentSwitch]);

  // Clear messages
  const clearMessages = useCallback(() => {
    setMessages([]);
    streamingMessageRef.current = null;
    setIsLoading(false);
  }, []);

  // Start new thread
  const startNewThread = useCallback(() => {
    clearMessages();
    setThreadId(`thread-${Date.now()}`);
  }, [clearMessages]);

  // Track used agents
  const usedAgents = Array.from(
    new Set(
      messages
        .filter((msg) => msg.role === 'assistant' && msg.agentId)
        .map((msg) => msg.agentId)
    )
  ).filter(Boolean) as string[];

  // Update selected agent
  const handleSetSelectedAgentId = useCallback((agentId: string | null) => {
    const previousAgent = selectedAgentId;
    setSelectedAgentId(agentId);

    if (agentId && previousAgent !== agentId && messages.length === 0) {
      // First agent selection
      const systemMessage: ChatMessage = {
        id: `system-${Date.now()}`,
        role: 'system',
        content: `✨ Started conversation with ${agentId}`,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, systemMessage]);
      previousAgentRef.current = agentId;
    }
  }, [selectedAgentId, messages.length]);

  return {
    messages,
    sendMessage,
    clearMessages,
    isConnected,
    isLoading,
    threadId,
    selectedAgentId,
    setSelectedAgentId: handleSetSelectedAgentId,
    startNewThread,
    usedAgents,
  };
}
