import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  Settings2,
  ChevronRight,
  Cpu,
  Thermometer,
  MessageSquare,
  Wrench,
  Zap,
  RotateCcw,
} from "lucide-react";

interface Tool {
  id: string;
  name: string;
  enabled: boolean;
  description: string;
}

export interface ChatSettings {
  model: string;
  temperature: number;
  maxTokens: number;
  systemPrompt: string;
  tools: Tool[];
  streamResponse: boolean;
  topP: number;
}

interface ChatSettingsPanelProps {
  isOpen: boolean;
  onToggle: () => void;
  settings: ChatSettings;
  onSettingsChange: (settings: ChatSettings) => void;
}

const defaultTools: Tool[] = [
  { id: "web_search", name: "Web Search", enabled: true, description: "Search the web for information" },
  { id: "code_interpreter", name: "Code Interpreter", enabled: true, description: "Execute and analyze code" },
  { id: "file_reader", name: "File Reader", enabled: false, description: "Read and parse files" },
  { id: "api_caller", name: "API Caller", enabled: false, description: "Make external API calls" },
  { id: "database", name: "Database Query", enabled: false, description: "Query databases" },
];

const models = [
  { id: "gpt-4o", name: "GPT-4o", provider: "OpenAI" },
  { id: "gpt-4o-mini", name: "GPT-4o Mini", provider: "OpenAI" },
  { id: "claude-3-opus", name: "Claude 3 Opus", provider: "Anthropic" },
  { id: "claude-3-sonnet", name: "Claude 3 Sonnet", provider: "Anthropic" },
  { id: "gemini-pro", name: "Gemini Pro", provider: "Google" },
  { id: "llama-3-70b", name: "Llama 3 70B", provider: "Meta" },
];

const ChatSettingsPanel = ({
  isOpen,
  onToggle,
  settings,
  onSettingsChange,
}: ChatSettingsPanelProps) => {
  const [expandedSections, setExpandedSections] = useState({
    model: true,
    parameters: true,
    systemPrompt: false,
    tools: false,
    advanced: false,
  });

  const toggleSection = (section: keyof typeof expandedSections) => {
    setExpandedSections((prev) => ({ ...prev, [section]: !prev[section] }));
  };

  const handleToolToggle = (toolId: string) => {
    const updatedTools = settings.tools.map((tool) =>
      tool.id === toolId ? { ...tool, enabled: !tool.enabled } : tool
    );
    onSettingsChange({ ...settings, tools: updatedTools });
  };

  const handleReset = () => {
    onSettingsChange({
      model: "gpt-4o",
      temperature: 0.7,
      maxTokens: 4096,
      systemPrompt: "You are a helpful AI assistant.",
      tools: defaultTools,
      streamResponse: true,
      topP: 1,
    });
  };

  if (!isOpen) {
    return (
      <Button
        variant="ghost"
        size="icon"
        onClick={onToggle}
        className="absolute right-4 top-4 h-10 w-10 rounded-full bg-muted/50 hover:bg-muted"
      >
        <Settings2 className="h-5 w-5" />
      </Button>
    );
  }

  return (
    <div className="w-80 border-l border-border/50 bg-background/50 backdrop-blur-sm flex flex-col h-full">
      <div className="flex items-center justify-between p-4 border-b border-border/30">
        <h3 className="font-semibold text-sm">Chat Settings</h3>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="icon" onClick={handleReset} className="h-8 w-8">
            <RotateCcw className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="icon" onClick={onToggle} className="h-8 w-8">
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-4 space-y-4">
          {/* Model Selection */}
          <Collapsible open={expandedSections.model} onOpenChange={() => toggleSection("model")}>
            <CollapsibleTrigger className="flex items-center justify-between w-full py-2 text-sm font-medium hover:text-primary transition-colors">
              <div className="flex items-center gap-2">
                <Cpu className="h-4 w-4 text-primary" />
                <span>Model</span>
              </div>
              <ChevronRight
                className={`h-4 w-4 transition-transform ${expandedSections.model ? "rotate-90" : ""}`}
              />
            </CollapsibleTrigger>
            <CollapsibleContent className="pt-3 space-y-3">
              <Select
                value={settings.model}
                onValueChange={(value) => onSettingsChange({ ...settings, model: value })}
              >
                <SelectTrigger className="w-full bg-muted/30 border-border/50">
                  <SelectValue placeholder="Select model" />
                </SelectTrigger>
                <SelectContent className="bg-background border-border">
                  {models.map((model) => (
                    <SelectItem key={model.id} value={model.id}>
                      <div className="flex items-center justify-between gap-4">
                        <span>{model.name}</span>
                        <span className="text-xs text-muted-foreground">{model.provider}</span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </CollapsibleContent>
          </Collapsible>

          {/* Parameters */}
          <Collapsible open={expandedSections.parameters} onOpenChange={() => toggleSection("parameters")}>
            <CollapsibleTrigger className="flex items-center justify-between w-full py-2 text-sm font-medium hover:text-primary transition-colors">
              <div className="flex items-center gap-2">
                <Thermometer className="h-4 w-4 text-primary" />
                <span>Parameters</span>
              </div>
              <ChevronRight
                className={`h-4 w-4 transition-transform ${expandedSections.parameters ? "rotate-90" : ""}`}
              />
            </CollapsibleTrigger>
            <CollapsibleContent className="pt-3 space-y-4">
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label className="text-xs text-muted-foreground">Temperature</Label>
                  <span className="text-xs font-mono bg-muted/50 px-2 py-0.5 rounded">
                    {settings.temperature.toFixed(2)}
                  </span>
                </div>
                <Slider
                  value={[settings.temperature]}
                  onValueChange={([value]) => onSettingsChange({ ...settings, temperature: value })}
                  min={0}
                  max={2}
                  step={0.01}
                  className="w-full"
                />
                <p className="text-[10px] text-muted-foreground">
                  Lower = more focused, Higher = more creative
                </p>
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label className="text-xs text-muted-foreground">Max Tokens</Label>
                  <span className="text-xs font-mono bg-muted/50 px-2 py-0.5 rounded">
                    {settings.maxTokens}
                  </span>
                </div>
                <Slider
                  value={[settings.maxTokens]}
                  onValueChange={([value]) => onSettingsChange({ ...settings, maxTokens: value })}
                  min={256}
                  max={16384}
                  step={256}
                  className="w-full"
                />
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label className="text-xs text-muted-foreground">Top P</Label>
                  <span className="text-xs font-mono bg-muted/50 px-2 py-0.5 rounded">
                    {settings.topP.toFixed(2)}
                  </span>
                </div>
                <Slider
                  value={[settings.topP]}
                  onValueChange={([value]) => onSettingsChange({ ...settings, topP: value })}
                  min={0}
                  max={1}
                  step={0.01}
                  className="w-full"
                />
              </div>
            </CollapsibleContent>
          </Collapsible>

          {/* System Prompt */}
          <Collapsible open={expandedSections.systemPrompt} onOpenChange={() => toggleSection("systemPrompt")}>
            <CollapsibleTrigger className="flex items-center justify-between w-full py-2 text-sm font-medium hover:text-primary transition-colors">
              <div className="flex items-center gap-2">
                <MessageSquare className="h-4 w-4 text-primary" />
                <span>System Prompt</span>
              </div>
              <ChevronRight
                className={`h-4 w-4 transition-transform ${expandedSections.systemPrompt ? "rotate-90" : ""}`}
              />
            </CollapsibleTrigger>
            <CollapsibleContent className="pt-3">
              <Textarea
                value={settings.systemPrompt}
                onChange={(e) => onSettingsChange({ ...settings, systemPrompt: e.target.value })}
                placeholder="Enter system prompt..."
                className="min-h-[120px] bg-muted/30 border-border/50 text-sm resize-none"
              />
            </CollapsibleContent>
          </Collapsible>

          {/* Tools */}
          <Collapsible open={expandedSections.tools} onOpenChange={() => toggleSection("tools")}>
            <CollapsibleTrigger className="flex items-center justify-between w-full py-2 text-sm font-medium hover:text-primary transition-colors">
              <div className="flex items-center gap-2">
                <Wrench className="h-4 w-4 text-primary" />
                <span>Tools</span>
                <span className="text-xs text-muted-foreground">
                  ({settings.tools.filter((t) => t.enabled).length} active)
                </span>
              </div>
              <ChevronRight
                className={`h-4 w-4 transition-transform ${expandedSections.tools ? "rotate-90" : ""}`}
              />
            </CollapsibleTrigger>
            <CollapsibleContent className="pt-3 space-y-2">
              {settings.tools.map((tool) => (
                <div
                  key={tool.id}
                  className="flex items-center justify-between p-2 rounded-lg bg-muted/20 hover:bg-muted/30 transition-colors"
                >
                  <div className="flex-1">
                    <p className="text-sm font-medium">{tool.name}</p>
                    <p className="text-[10px] text-muted-foreground">{tool.description}</p>
                  </div>
                  <Switch
                    checked={tool.enabled}
                    onCheckedChange={() => handleToolToggle(tool.id)}
                  />
                </div>
              ))}
            </CollapsibleContent>
          </Collapsible>

          {/* Advanced */}
          <Collapsible open={expandedSections.advanced} onOpenChange={() => toggleSection("advanced")}>
            <CollapsibleTrigger className="flex items-center justify-between w-full py-2 text-sm font-medium hover:text-primary transition-colors">
              <div className="flex items-center gap-2">
                <Zap className="h-4 w-4 text-primary" />
                <span>Advanced</span>
              </div>
              <ChevronRight
                className={`h-4 w-4 transition-transform ${expandedSections.advanced ? "rotate-90" : ""}`}
              />
            </CollapsibleTrigger>
            <CollapsibleContent className="pt-3 space-y-3">
              <div className="flex items-center justify-between p-2 rounded-lg bg-muted/20">
                <div>
                  <p className="text-sm font-medium">Stream Response</p>
                  <p className="text-[10px] text-muted-foreground">Show tokens as they generate</p>
                </div>
                <Switch
                  checked={settings.streamResponse}
                  onCheckedChange={(checked) =>
                    onSettingsChange({ ...settings, streamResponse: checked })
                  }
                />
              </div>
            </CollapsibleContent>
          </Collapsible>
        </div>
      </ScrollArea>
    </div>
  );
};

export default ChatSettingsPanel;
