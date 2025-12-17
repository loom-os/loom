import { useState, useCallback, useRef, useEffect } from 'react';
import { useWebSocket, type WsMessage } from './useWebSocket';
import { useChatContext } from '@/contexts/ChatContext';

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

  // Use global chat context for persistent state
  const {
    messages,
    selectedAgentId,
    threadId,
    usedAgents,
    streamingMessageId,
    addMessage,
    updateMessage,
    setSelectedAgentId,
    setStreamingMessageId,
    clearMessages: contextClearMessages,
    startNewThread: contextStartNewThread,
  } = useChatContext();

  const [isLoading, setIsLoading] = useState(false);

  // Refs for streaming accumulation
  const streamingMessageRef = useRef<{ id: string; content: string; contentType: ContentType } | null>(null);
  const previousAgentRef = useRef<string | null>(null);

  // Restore streaming state on mount
  useEffect(() => {
    if (streamingMessageId) {
      const streamingMsg = messages.find(m => m.id === streamingMessageId && m.isStreaming);
      if (streamingMsg) {
        streamingMessageRef.current = {
          id: streamingMsg.id,
          content: streamingMsg.content,
          contentType: streamingMsg.contentType || 'text',
        };
        setIsLoading(true);
        console.log('[useChat] Restored streaming state for message:', streamingMessageId);
      } else {
        // Message completed while we were away
        setStreamingMessageId(null);
      }
    }
  }, []); // Only run once on mount

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

          addMessage(newMessage);
          setStreamingMessageId(messageId);
        } else {
          // Subsequent chunks - append content
          streamingMessageRef.current.content += content;
          const messageId = streamingMessageRef.current.id;
          const fullContent = streamingMessageRef.current.content;
          const storedContentType = streamingMessageRef.current.contentType;

          updateMessage(messageId, {
            content: fullContent,
            isStreaming: true,
            toolCall: storedContentType === 'tool_call' ? parseToolCall(fullContent) : undefined,
          });
        }
        break;
      }

      case 'chat_complete': {
        const { thread_id, stats } = lastMessage;
        if (thread_id !== threadId) return;

        // Mark message as completed (no longer streaming)
        if (streamingMessageRef.current) {
          const messageId = streamingMessageRef.current.id;
          updateMessage(messageId, {
            isStreaming: false,
            metadata: stats
          });
        }

        // Finish streaming
        streamingMessageRef.current = null;
        setStreamingMessageId(null);
        setIsLoading(false);
        break;
      }

      case 'error': {
        console.error('[useChat] Error:', lastMessage.message);
        streamingMessageRef.current = null;
        setStreamingMessageId(null);
        setIsLoading(false);

        // Show error message
        const errorMessage: ChatMessage = {
          id: `error-${Date.now()}`,
          role: 'assistant',
          content: lastMessage.message || 'An error occurred',
          contentType: 'error',
          timestamp: new Date(),
        };
        addMessage(errorMessage);

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
        addMessage(systemMessage);
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
    addMessage(userMessage);
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
    contextClearMessages();
    streamingMessageRef.current = null;
    setStreamingMessageId(null);
    setIsLoading(false);
  }, [contextClearMessages, setStreamingMessageId]);

  // Start new thread
  const startNewThread = useCallback(() => {
    contextStartNewThread();
    streamingMessageRef.current = null;
    setIsLoading(false);
  }, [contextStartNewThread]);

  // usedAgents now comes from context

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
      addMessage(systemMessage);
      previousAgentRef.current = agentId;
    }
  }, [selectedAgentId, messages.length, setSelectedAgentId, addMessage]);

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
