import { useState, useRef } from "react";
import ChatInterface from "@/components/ChatInterface";
import ChatSettingsPanel, { ChatSettings } from "@/components/ChatSettingsPanel";
import AgentSelector from "@/components/AgentSelector";
import { Settings2, Wifi, WifiOff, Users } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { useChat } from "@/hooks/useChat";

const defaultTools = [
  { id: "web_search", name: "Web Search", enabled: true, description: "Search the web for information" },
  { id: "code_interpreter", name: "Code Interpreter", enabled: true, description: "Execute and analyze code" },
  { id: "file_reader", name: "File Reader", enabled: false, description: "Read and parse files" },
  { id: "api_caller", name: "API Caller", enabled: false, description: "Make external API calls" },
  { id: "database", name: "Database Query", enabled: false, description: "Query databases" },
];

const ChatPage = () => {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [agentSelectorOpen, setAgentSelectorOpen] = useState(false);
  const previousAgentIdRef = useRef<string | null>(null);
  const [settings, setSettings] = useState<ChatSettings>({
    model: "gpt-4o",
    temperature: 0.7,
    maxTokens: 4096,
    systemPrompt: "You are a helpful AI assistant.",
    tools: defaultTools,
    streamResponse: true,
    topP: 1,
  });

  // Use the unified chat hook
  const {
    messages,
    isLoading,
    isConnected,
    selectedAgentId,
    setSelectedAgentId,
    usedAgents,
    sendMessage,
    clearMessages,
    startNewThread,
  } = useChat();

  const handleSendMessage = (content: string) => {
    if (!isConnected) {
      console.warn('[ChatPage] Cannot send message: not connected');
      return;
    }

    if (!selectedAgentId) {
      console.warn('[ChatPage] Cannot send message: no agent selected');
      setAgentSelectorOpen(true);
      return;
    }

    sendMessage(content, selectedAgentId, {
      model: settings.model,
      temperature: settings.temperature,
      maxTokens: settings.maxTokens,
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
              {selectedAgentId ? (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setAgentSelectorOpen(true)}
                  className="flex items-center gap-2 pl-3 border-l h-8"
                >
                  <Users className="h-4 w-4 text-primary" />
                  <span className="text-sm font-medium">{selectedAgentId}</span>
                  <span className="text-xs text-muted-foreground">(click to change)</span>
                </Button>
              ) : (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setAgentSelectorOpen(true)}
                  className="flex items-center gap-2 pl-3 border-l h-8 text-amber-600"
                >
                  <Users className="h-4 w-4" />
                  <span className="text-sm font-medium">Select an agent</span>
                </Button>
              )}
            </div>
            <div className="flex items-center gap-3">
              {usedAgents.length > 1 && (
                <Badge variant="secondary" className="text-xs">
                  {usedAgents.length} agents used
                </Badge>
              )}
              {messages.length > 0 && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    clearMessages();
                    startNewThread();
                  }}
                  className="h-7 text-xs"
                >
                  New Chat
                </Button>
              )}
            </div>
          </div>

          {/* Chat Interface */}
          <div className="flex-1 overflow-hidden">
            <ChatInterface
              messages={messages}
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
