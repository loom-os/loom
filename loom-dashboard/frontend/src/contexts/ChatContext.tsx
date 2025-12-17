/**
 * Chat Context Provider
 *
 * Provides global chat state that persists across page navigation
 */

import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import { ChatMessage, ChatSettings } from '@/hooks/useChat';

interface ChatState {
  messages: ChatMessage[];
  selectedAgentId: string | null;
  threadId: string;
  usedAgents: string[];
  streamingMessageId: string | null;
}

interface ChatContextValue {
  messages: ChatMessage[];
  selectedAgentId: string | null;
  threadId: string;
  usedAgents: string[];
  streamingMessageId: string | null;
  addMessage: (message: ChatMessage) => void;
  updateMessage: (id: string, updates: Partial<ChatMessage>) => void;
  setSelectedAgentId: (agentId: string | null) => void;
  setStreamingMessageId: (id: string | null) => void;
  clearMessages: () => void;
  startNewThread: () => void;
}

const ChatContext = createContext<ChatContextValue | null>(null);

const STORAGE_KEY = 'loom-chat-state';

function loadState(): ChatState {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      // Convert timestamp strings back to Date objects
      if (parsed.messages) {
        parsed.messages = parsed.messages.map((msg: any) => ({
          ...msg,
          timestamp: new Date(msg.timestamp),
        }));
      }
      return parsed;
    }
  } catch (error) {
    console.error('[ChatContext] Failed to load state:', error);
  }

  return {
    messages: [],
    selectedAgentId: null,
    threadId: `thread-${Date.now()}`,
    usedAgents: [],
    streamingMessageId: null,
  };
}

function saveState(state: ChatState) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch (error) {
    console.error('[ChatContext] Failed to save state:', error);
  }
}

export function ChatProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<ChatState>(loadState);

  // Save to localStorage whenever state changes
  useEffect(() => {
    saveState(state);
  }, [state]);

  const addMessage = useCallback((message: ChatMessage) => {
    setState((prev) => {
      const newMessages = [...prev.messages, message];
      const newUsedAgents = message.agentId && !prev.usedAgents.includes(message.agentId)
        ? [...prev.usedAgents, message.agentId]
        : prev.usedAgents;

      return {
        ...prev,
        messages: newMessages,
        usedAgents: newUsedAgents,
      };
    });
  }, []);

  const updateMessage = useCallback((id: string, updates: Partial<ChatMessage>) => {
    setState((prev) => ({
      ...prev,
      messages: prev.messages.map((msg) =>
        msg.id === id ? { ...msg, ...updates } : msg
      ),
    }));
  }, []);

  const setSelectedAgentId = useCallback((agentId: string | null) => {
    setState((prev) => ({
      ...prev,
      selectedAgentId: agentId,
    }));
  }, []);

  const setStreamingMessageId = useCallback((id: string | null) => {
    setState((prev) => ({
      ...prev,
      streamingMessageId: id,
    }));
  }, []);

  const clearMessages = useCallback(() => {
    setState((prev) => ({
      ...prev,
      messages: [],
      usedAgents: [],
      streamingMessageId: null,
    }));
  }, []);

  const startNewThread = useCallback(() => {
    setState((prev) => ({
      ...prev,
      threadId: `thread-${Date.now()}`,
      messages: [],
      usedAgents: [],
      streamingMessageId: null,
    }));
  }, []);

  const value: ChatContextValue = {
    messages: state.messages,
    selectedAgentId: state.selectedAgentId,
    threadId: state.threadId,
    usedAgents: state.usedAgents,
    streamingMessageId: state.streamingMessageId,
    addMessage,
    updateMessage,
    setSelectedAgentId,
    setStreamingMessageId,
    clearMessages,
    startNewThread,
  };

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
}

export function useChatContext() {
  const context = useContext(ChatContext);
  if (!context) {
    throw new Error('useChatContext must be used within ChatProvider');
  }
  return context;
}
