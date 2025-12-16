import { useState } from "react";
import ChatInterface from "@/components/ChatInterface";
import ChatSettingsPanel, { ChatSettings } from "@/components/ChatSettingsPanel";
import { Settings2 } from "lucide-react";
import { Button } from "@/components/ui/button";

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
  const [settings, setSettings] = useState<ChatSettings>({
    model: "gpt-4o",
    temperature: 0.7,
    maxTokens: 4096,
    systemPrompt: "You are a helpful AI assistant.",
    tools: defaultTools,
    streamResponse: true,
    topP: 1,
  });

  const handleSendMessage = (content: string) => {
    const userMessage: ChatMessage = {
      id: Math.random().toString(36).substr(2, 9),
      role: "user",
      content,
      timestamp: new Date(),
    };
    setChatMessages((prev) => [...prev, userMessage]);
    setIsLoading(true);

    // Simulate agent response
    setTimeout(() => {
      const respondingAgent = ["planner", "researcher", "writer"][
        Math.floor(Math.random() * 3)
      ];

      setTimeout(() => {
        const assistantMessage: ChatMessage = {
          id: Math.random().toString(36).substr(2, 9),
          role: "assistant",
          content: generateAgentResponse(content),
          timestamp: new Date(),
          agentId: respondingAgent,
        };
        setChatMessages((prev) => [...prev, assistantMessage]);
        setIsLoading(false);
      }, 1000 + Math.random() * 1000);
    }, 500 + Math.random() * 500);
  };

  const generateAgentResponse = (input: string): string => {
    const responses = [
      `I've analyzed your request about "${input.slice(0, 20)}...". The research agent is gathering relevant data, and I'm coordinating the workflow.`,
      `Processing your query. I've dispatched tasks to the research team and will synthesize the findings shortly.`,
      `Understood. I'm breaking down this task and routing it through our agent network for optimal processing.`,
      `Your request has been received. Multiple agents are collaborating to provide a comprehensive response.`,
    ];
    return responses[Math.floor(Math.random() * responses.length)];
  };

  return (
    <div className="h-full flex relative">
      <div className="flex-1 p-6">
        <div className="h-full max-w-4xl mx-auto">
          <ChatInterface
            messages={chatMessages}
            onSendMessage={handleSendMessage}
            isLoading={isLoading}
          />
        </div>
      </div>

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
