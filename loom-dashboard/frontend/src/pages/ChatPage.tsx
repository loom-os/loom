import { useState, useEffect, useRef } from "react";
import ChatInterface from "@/components/ChatInterface";
import ChatSettingsPanel, { ChatSettings } from "@/components/ChatSettingsPanel";
import AgentSelector from "@/components/AgentSelector";
import { Settings2, Wifi, WifiOff, Users } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useWebSocket, type WsMessage } from "@/hooks/useWebSocket";
import { Badge } from "@/components/ui/badge";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";

interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: Date;
  agentId?: string;
}

const defaultTools = [
  { id: "web_search", name: "Web Search", enabled: true, description: "Search the web for information" },
  { id: "code_interpreter", name: "Code Interpreter", enabled: true, description: "Execute and analyze code" },
  { id: "file_reader", name: "File Reader", enabled: false, description: "Read and parse files" },
  { id: "api_caller", name: "API Caller", enabled: false, description: "Make external API calls" },
  { id: "database", name: "Database Query", enabled: false, description: "Query databases" },
];

const ChatPage = () => {
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [agentSelectorOpen, setAgentSelectorOpen] = useState(false);
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [settings, setSettings] = useState<ChatSettings>({
    model: "gpt-4o",
    temperature: 0.7,
    maxTokens: 4096,
    systemPrompt: "You are a helpful AI assistant.",
    tools: defaultTools,
    streamResponse: true,
    topP: 1,
  });

  // Current thread ID for this chat session
  const threadIdRef = useRef<string>(`thread-${Date.now()}`);

  // Accumulate streaming response
  const streamingMessageRef = useRef<{ id: string; content: string } | null>(null);

  // WebSocket connection
  const wsUrl = `ws://${window.location.host}/ws`;
  const { isConnected, send, lastMessage } = useWebSocket(wsUrl, {
    reconnect: true,
    onOpen: () => {
      console.log('[ChatPage] WebSocket connected');
    },
    onClose: () => {
      console.log('[ChatPage] WebSocket disconnected');
    },
  });

  // Handle incoming messages
  useEffect(() => {
    if (!lastMessage) return;

    switch (lastMessage.type) {
      case 'chat_chunk': {
        const { thread_id, content, sequence } = lastMessage;
        if (thread_id !== threadIdRef.current) return;

        // Create or update streaming message
        if (sequence === 0 || !streamingMessageRef.current) {
          // First chunk - create new message
          const messageId = `msg-${Date.now()}`;
          streamingMessageRef.current = { id: messageId, content };

          const newMessage: ChatMessage = {
            id: messageId,
            role: "assistant",
            content,
            timestamp: new Date(),
          };
          setChatMessages((prev) => [...prev, newMessage]);
        } else {
          // Subsequent chunks - append content
          streamingMessageRef.current.content += content;
          const messageId = streamingMessageRef.current.id;

          setChatMessages((prev) =>
            prev.map((msg) =>
              msg.id === messageId
                ? { ...msg, content: streamingMessageRef.current!.content }
                : msg
            )
          );
        }
        break;
      }

      case 'chat_complete': {
        const { thread_id } = lastMessage;
        if (thread_id !== threadIdRef.current) return;

        // Finish streaming
        streamingMessageRef.current = null;
        setIsLoading(false);
        break;
      }

      case 'error': {
        console.error('[ChatPage] Error:', lastMessage.message);
        setIsLoading(false);

        // Show error message
        const errorMessage: ChatMessage = {
          id: `error-${Date.now()}`,
          role: "assistant",
          content: `Error: ${lastMessage.message}`,
          timestamp: new Date(),
        };
        setChatMessages((prev) => [...prev, errorMessage]);
        break;
      }
    }
  }, [lastMessage]);

  const handleSendMessage = (content: string) => {
    if (!isConnected) {
      console.warn('[ChatPage] Cannot send message: not connected');
      return;
    }

    if (!selectedAgentId) {
      console.warn('[ChatPage] Cannot send message: no agent selected');
      // Show error in chat
      const errorMessage: ChatMessage = {
        id: `error-${Date.now()}`,
        role: "assistant",
        content: "⚠️ Please select an agent first",
        timestamp: new Date(),
      };
      setChatMessages((prev) => [...prev, errorMessage]);
      setAgentSelectorOpen(true);
      return;
    }

    // Add user message
    const userMessage: ChatMessage = {
      id: `user-${Date.now()}`,
      role: "user",
      content,
      timestamp: new Date(),
      agentId: selectedAgentId,
    };
    setChatMessages((prev) => [...prev, userMessage]);
    setIsLoading(true);

    // Send to WebSocket with selected agent
    send({
      type: 'chat_request',
      thread_id: threadIdRef.current,
      content,
      agent_id: selectedAgentId,
      settings: {
        model: settings.model,
        temperature: settings.temperature,
        max_tokens: settings.maxTokens,
      },
    });
  };

  return (
    <div className="h-full flex relative">
      <div className="flex-1 p-6">
        <div className="h-full max-w-4xl mx-auto flex flex-col gap-4">
          {/* Status Bar */}
          <div className="flex items-center justify-between px-4 py-2 bg-muted/30 rounded-lg">
            <div className="flex items-center gap-3">
              {/* Connection Status */}
              <div className="flex items-center gap-2">
                {isConnected ? (
                  <>
                    <Wifi className="h-4 w-4 text-green-500" />
                    <Badge variant="outline" className="text-green-600 border-green-600">
                      Connected
                    </Badge>
                  </>
                ) : (
                  <>
                    <WifiOff className="h-4 w-4 text-amber-500" />
                    <Badge variant="outline" className="text-amber-600 border-amber-600">
                      Connecting...
                    </Badge>
                  </>
                )}
              </div>

              {/* Selected Agent */}
              {selectedAgentId && (
                <div className="flex items-center gap-2 pl-3 border-l">
                  <Users className="h-4 w-4 text-primary" />
                  <span className="text-sm font-medium">{selectedAgentId}</span>
                </div>
              )}
            </div>
            <p className="text-sm text-muted-foreground">
              Thread: {threadIdRef.current}
            </p>
          </div>

          {/* Chat Interface */}
          <div className="flex-1 overflow-hidden">
            <ChatInterface
              messages={chatMessages}
              onSendMessage={handleSendMessage}
              isLoading={isLoading}
            />
          </div>
        </div>
      </div>

      {/* Agent Selector Button */}
      <Sheet open={agentSelectorOpen} onOpenChange={setAgentSelectorOpen}>
        <SheetTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="absolute left-4 top-4 h-10 w-10 rounded-full bg-muted/50 hover:bg-muted"
          >
            <Users className="h-5 w-5" />
          </Button>
        </SheetTrigger>
        <SheetContent side="left" className="w-[400px] sm:w-[540px]">
          <SheetHeader>
            <SheetTitle>Select Agent</SheetTitle>
          </SheetHeader>
          <div className="mt-6">
            <AgentSelector
              selectedAgentId={selectedAgentId}
              onSelectAgent={(agentId) => {
                setSelectedAgentId(agentId);
                setAgentSelectorOpen(false);
              }}
              cognitiveOnly={true}
            />
          </div>
        </SheetContent>
      </Sheet>

      {/* Settings Button */}
      {!settingsOpen && (
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setSettingsOpen(true)}
          className="absolute right-4 top-4 h-10 w-10 rounded-full bg-muted/50 hover:bg-muted"
        >
          <Settings2 className="h-5 w-5" />
        </Button>
      )}

      {settingsOpen && (
        <ChatSettingsPanel
          isOpen={settingsOpen}
          onToggle={() => setSettingsOpen(false)}
          settings={settings}
          onSettingsChange={setSettings}
        />
      )}
    </div>
  );
};

export default ChatPage;
