import { useState, useRef, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Send, Bot } from "lucide-react";
import { MessageList } from "@/components/MessageBubble";
import type { ChatMessage } from "@/hooks/useChat";

interface ChatInterfaceProps {
  onSendMessage?: (message: string) => void;
  messages: ChatMessage[];
  isLoading?: boolean;
}

const ChatInterface = ({ onSendMessage, messages, isLoading = false }: ChatInterfaceProps) => {
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const handleSend = () => {
    if (input.trim() && onSendMessage) {
      onSendMessage(input.trim());
      setInput("");
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const hasMessages = messages.length > 0;

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {!hasMessages ? (
        // Centered welcome state
        <div className="flex-1 flex flex-col items-center justify-center px-4">
          <div className="w-16 h-16 rounded-full bg-primary/10 flex items-center justify-center mb-6">
            <Bot className="h-8 w-8 text-primary" />
          </div>
          <h2 className="text-2xl font-semibold text-foreground mb-2">Agent Chat</h2>
          <p className="text-muted-foreground text-center max-w-md mb-8">
            Chat with intelligent agents to accomplish your tasks
          </p>
          <div className="w-full max-w-2xl">
            <div className="flex gap-3">
              <Input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Ask me anything..."
                className="flex-1 h-12 bg-muted/30 border-border/50 text-base px-4"
                disabled={isLoading}
              />
              <Button
                onClick={handleSend}
                disabled={!input.trim() || isLoading}
                size="lg"
                className="h-12 px-6 bg-primary hover:bg-primary/90"
              >
                <Send className="h-5 w-5" />
              </Button>
            </div>
          </div>
        </div>
      ) : (
        // Conversation state
        <>
          <ScrollArea className="flex-1 px-4 py-4" ref={scrollRef}>
            <div className="max-w-3xl mx-auto">
              <MessageList messages={messages} isLoading={isLoading} />
            </div>
          </ScrollArea>

          <div className="p-4 border-t border-border/30">
            <div className="flex gap-3 max-w-3xl mx-auto">
              <Input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Message agents..."
                className="flex-1 h-11 bg-muted/30 border-border/50 text-sm px-4"
                disabled={isLoading}
              />
              <Button
                onClick={handleSend}
                disabled={!input.trim() || isLoading}
                size="default"
                className="h-11 px-5 bg-primary hover:bg-primary/90"
              >
                <Send className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </>
      )}
    </div>
  );
};

export default ChatInterface;
