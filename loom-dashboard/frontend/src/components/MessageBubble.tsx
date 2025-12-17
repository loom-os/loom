import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import rehypeRaw from 'rehype-raw';
import remarkGfm from 'remark-gfm';
import { Bot, User, Brain, Wrench, CheckCircle2, AlertCircle, Loader2, ChevronDown, ChevronRight, Copy, RotateCcw, X } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ChatMessage, ContentType } from '@/hooks/useChat';
import 'highlight.js/styles/github-dark.css';

interface MessageBubbleProps {
  message: ChatMessage;
  onCancel?: () => void;
  onRetry?: () => void;
}

/**
 * Message action buttons (Cancel/Retry/Copy)
 */
function MessageActions({
  content,
  isStreaming,
  onCancel,
  onRetry
}: {
  content: string;
  isStreaming?: boolean;
  onCancel?: () => void;
  onRetry?: () => void;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return (
    <div className="flex items-center gap-2 mt-3 opacity-0 group-hover:opacity-100 transition-opacity">
      {isStreaming && onCancel && (
        <button
          onClick={onCancel}
          className="inline-flex items-center gap-1.5 px-2 py-1 text-xs text-red-400 hover:text-red-300 hover:bg-red-500/10 rounded transition-colors"
          title="Cancel generation"
        >
          <X className="h-3 w-3" />
          <span>Cancel</span>
        </button>
      )}

      {!isStreaming && (
        <>
          {onRetry && (
            <button
              onClick={onRetry}
              className="inline-flex items-center gap-1.5 px-2 py-1 text-xs text-muted-foreground hover:text-foreground hover:bg-muted/50 rounded transition-colors"
              title="Retry generation"
            >
              <RotateCcw className="h-3 w-3" />
              <span>Retry</span>
            </button>
          )}

          <button
            onClick={handleCopy}
            className="inline-flex items-center gap-1.5 px-2 py-1 text-xs text-muted-foreground hover:text-foreground hover:bg-muted/50 rounded transition-colors"
            title="Copy to clipboard"
          >
            <Copy className="h-3 w-3" />
            <span>{copied ? 'Copied!' : 'Copy'}</span>
          </button>
        </>
      )}
    </div>
  );
}

const ContentTypeIcon: Record<ContentType, React.ReactNode> = {
  text: null,
  thinking: <Brain className="h-3.5 w-3.5" />,
  tool_call: <Wrench className="h-3.5 w-3.5" />,
  tool_result: <CheckCircle2 className="h-3.5 w-3.5" />,
  error: <AlertCircle className="h-3.5 w-3.5" />,
};

const ContentTypeLabel: Record<ContentType, string> = {
  text: '',
  thinking: 'Thinking',
  tool_call: 'Tool Call',
  tool_result: 'Result',
  error: 'Error',
};

/**
 * Parse ReAct pattern markers from content
 * Handles: Thought:, Action:, Observation:, FINAL ANSWER:
 */
interface ParsedSection {
  type: 'thought' | 'action' | 'observation' | 'final' | 'text';
  content: string;
}

function parseReActContent(content: string): ParsedSection[] {
  const sections: ParsedSection[] = [];

  // Split by section markers, keeping the markers
  const markers = ['Thought:', 'Action:', 'Observation:', 'FINAL ANSWER:'];

  // Find all marker positions
  const positions: Array<{ index: number; marker: string; type: ParsedSection['type'] }> = [];

  for (const marker of markers) {
    let index = 0;
    while ((index = content.indexOf(marker, index)) !== -1) {
      const type = marker === 'Thought:' ? 'thought'
                 : marker === 'Action:' ? 'action'
                 : marker === 'Observation:' ? 'observation'
                 : 'final';
      positions.push({ index, marker, type });
      index += marker.length;
    }
  }

  // Sort by position
  positions.sort((a, b) => a.index - b.index);

  // If no markers found, return as text
  if (positions.length === 0) {
    return [{ type: 'text', content: content.trim() }];
  }

  // Extract sections
  for (let i = 0; i < positions.length; i++) {
    const start = positions[i].index + positions[i].marker.length;
    const end = i < positions.length - 1 ? positions[i + 1].index : content.length;
    const sectionContent = content.substring(start, end).trim();

    if (sectionContent) {
      sections.push({
        type: positions[i].type,
        content: sectionContent
      });
    }
  }

  // Add any text before the first marker
  if (positions[0].index > 0) {
    const preText = content.substring(0, positions[0].index).trim();
    if (preText) {
      sections.unshift({ type: 'text', content: preText });
    }
  }

  return sections.filter(s => s.content.trim());
}

/**
 * Render content with ReAct pattern awareness
 */
function RenderContent({ content, isThinking }: { content: string; isThinking?: boolean }) {
  const sections = parseReActContent(content);

  // If no special markers, render as plain markdown
  if (sections.length === 1 && sections[0].type === 'text') {
    return (
      <div className={cn(
        'prose prose-sm dark:prose-invert max-w-none',
        'prose-p:leading-relaxed prose-p:text-foreground',
        'prose-headings:text-foreground prose-headings:font-semibold',
        'prose-strong:text-foreground prose-strong:font-semibold',
        'prose-code:text-foreground prose-code:bg-muted prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded prose-code:text-xs',
        'prose-pre:bg-muted prose-pre:border prose-pre:border-border',
        'prose-a:text-primary prose-a:no-underline hover:prose-a:underline',
        'prose-blockquote:border-l-primary prose-blockquote:text-muted-foreground',
        'prose-ul:text-foreground prose-ol:text-foreground',
        'prose-li:text-foreground prose-li:marker:text-muted-foreground',
        isThinking && 'prose-p:italic prose-p:text-purple-300'
      )}>
        <MarkdownContent content={content} />
      </div>
    );
  }

  // Render sections with styling
  return (
    <div className="space-y-3">
      {sections.map((section, idx) => (
        <RenderSection key={idx} section={section} />
      ))}
    </div>
  );
}

/**
 * Collapsible section component
 */
function CollapsibleSection({
  title,
  icon,
  children,
  colorClass,
  defaultOpen = false
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
  colorClass: string;
  defaultOpen?: boolean;
}) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className={cn('rounded-lg border p-3', colorClass)}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-2 w-full text-xs font-medium mb-2 hover:opacity-80 transition-opacity"
      >
        {isOpen ? (
          <ChevronDown className="h-3.5 w-3.5" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5" />
        )}
        {icon}
        <span>{title}</span>
        <span className="ml-auto text-[10px] opacity-60">
          {isOpen ? 'Click to collapse' : 'Click to expand'}
        </span>
      </button>
      {isOpen && children}
    </div>
  );
}

/**
 * Render a single ReAct section
 */
function RenderSection({ section }: { section: ParsedSection }) {
  switch (section.type) {
    case 'thought':
      return (
        <CollapsibleSection
          title="Thinking"
          icon={<Brain className="h-3.5 w-3.5" />}
          colorClass="bg-purple-500/5 border-purple-500/20 text-purple-400"
          defaultOpen={false}
        >
          <div className="text-sm text-purple-100/90 italic leading-relaxed">
            {section.content}
          </div>
        </CollapsibleSection>
      );

    case 'action':
      // Extract JSON if present, and separate from trailing text
      const parseActionContent = (content: string) => {
        // Try to find JSON object
        const jsonMatch = content.match(/\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}/);

        if (jsonMatch) {
          try {
            const json = JSON.parse(jsonMatch[0]);
            const beforeJson = content.substring(0, jsonMatch.index).trim();
            const afterJson = content.substring(jsonMatch.index! + jsonMatch[0].length).trim();
            return { json, beforeJson, afterJson };
          } catch {
            return { json: null, beforeJson: '', afterJson: content };
          }
        }

        return { json: null, beforeJson: '', afterJson: content };
      };

      const { json, beforeJson, afterJson } = parseActionContent(section.content);

      return (
        <CollapsibleSection
          title="Action"
          icon={<Wrench className="h-3.5 w-3.5" />}
          colorClass="bg-cyan-500/5 border-cyan-500/20 text-cyan-400"
          defaultOpen={false}
        >
          <div className="space-y-2">
            {beforeJson && (
              <div className="text-sm text-cyan-100/90">
                {beforeJson}
              </div>
            )}
            {json && (
              <pre className="text-xs bg-black/30 text-cyan-100 p-2 rounded overflow-x-auto border border-cyan-500/10">
                <code>{JSON.stringify(json, null, 2)}</code>
              </pre>
            )}
            {afterJson && !json && (
              <div className="text-sm text-cyan-100/90 font-mono whitespace-pre-wrap">
                {afterJson}
              </div>
            )}
          </div>
        </CollapsibleSection>
      );

    case 'observation':
      return (
        <CollapsibleSection
          title="Observation"
          icon={<CheckCircle2 className="h-3.5 w-3.5" />}
          colorClass="bg-green-500/5 border-green-500/20 text-green-400"
          defaultOpen={false}
        >
          <div className="text-sm text-green-100/90 whitespace-pre-wrap max-h-60 overflow-y-auto">
            {section.content.length > 1000 ? section.content.slice(0, 1000) + '\n\n... (truncated)' : section.content}
          </div>
        </CollapsibleSection>
      );

    case 'final':
      return (
        <div className="prose prose-sm dark:prose-invert max-w-none">
          <MarkdownContent content={section.content} />
        </div>
      );

    default:
      return (
        <div className="prose prose-sm dark:prose-invert max-w-none">
          <MarkdownContent content={section.content} />
        </div>
      );
  }
}

/**
 * Render Markdown content with custom components
 */
function MarkdownContent({ content }: { content: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      rehypePlugins={[rehypeHighlight, rehypeRaw]}
      components={{
        code({ className, children, ...props }: any) {
          const match = /language-(\w+)/.exec(className || '');
          const isInline = !match;

          if (isInline) {
            return (
              <code className="bg-muted px-1.5 py-0.5 rounded text-xs font-mono" {...props}>
                {children}
              </code>
            );
          }

          return (
            <code className={cn(className, 'text-xs')} {...props}>
              {children}
            </code>
          );
        },
        p({ children }) {
          return <p className="mb-4 last:mb-0 leading-7">{children}</p>;
        },
        a({ children, href }) {
          return (
            <a
              href={href}
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary hover:underline font-medium"
            >
              {children}
            </a>
          );
        },
        ul({ children }) {
          return <ul className="space-y-1 my-3">{children}</ul>;
        },
        ol({ children }) {
          return <ol className="space-y-1 my-3">{children}</ol>;
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
}

export function MessageBubble({ message, onCancel, onRetry }: MessageBubbleProps) {
  const { role, content, timestamp, agentId, agentName, contentType = 'text', isStreaming, toolCall } = message;

  // System messages (centered, compact)
  if (role === 'system') {
    return (
      <div className="flex justify-center my-6">
        <div className="text-xs text-muted-foreground bg-muted/30 px-4 py-2 rounded-full">
          {content}
        </div>
      </div>
    );
  }

  const isUser = role === 'user';
  const isThinking = contentType === 'thinking';
  const isToolCall = contentType === 'tool_call';
  const isToolResult = contentType === 'tool_result';
  const isError = contentType === 'error';

  // User message (right-aligned bubble)
  if (isUser) {
    return (
      <div className="flex gap-3 justify-end py-4">
        <div className="max-w-[70%] rounded-2xl bg-primary text-primary-foreground px-4 py-3 shadow-sm">
          <p className="whitespace-pre-wrap text-sm leading-relaxed">{content}</p>
        </div>
        <div className="w-8 h-8 rounded-full bg-secondary/10 flex items-center justify-center flex-shrink-0">
          <User className="h-4 w-4 text-secondary-foreground" />
        </div>
      </div>
    );
  }

  // AI message (full-width, no bubble - ChatGPT style)
  return (
    <div className="group py-6 px-4 hover:bg-muted/30 transition-colors">
      <div className="max-w-3xl mx-auto">
        <div className="flex gap-4">
          {/* Avatar */}
          <div className="flex-shrink-0">
            <div className="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center">
              <Bot className="h-4 w-4 text-primary" />
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0 space-y-3">
            {/* Header: Agent name + Content type badge + Streaming indicator */}
            <div className="flex items-center gap-2 flex-wrap">
              {(agentId || agentName) && (
                <span className="text-xs font-semibold text-foreground">
                  {agentName || agentId}
                </span>
              )}

              {contentType !== 'text' && (
                <span className={cn(
                  'inline-flex items-center gap-1 text-xs px-2 py-1 rounded-md font-medium',
                  isThinking && 'bg-purple-500/10 text-purple-400 border border-purple-500/20',
                  isToolCall && 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/20',
                  isToolResult && 'bg-green-500/10 text-green-400 border border-green-500/20',
                  isError && 'bg-red-500/10 text-red-400 border border-red-500/20'
                )}>
                  {ContentTypeIcon[contentType]}
                  <span>{ContentTypeLabel[contentType]}</span>
                </span>
              )}

              {isStreaming && (
                <div className="flex gap-1 ml-1">
                  <div className="w-1.5 h-1.5 rounded-full bg-primary/60 animate-pulse" style={{ animationDelay: '0ms' }}></div>
                  <div className="w-1.5 h-1.5 rounded-full bg-primary/60 animate-pulse" style={{ animationDelay: '150ms' }}></div>
                  <div className="w-1.5 h-1.5 rounded-full bg-primary/60 animate-pulse" style={{ animationDelay: '300ms' }}></div>
                </div>
              )}

              <span className="text-[10px] text-muted-foreground ml-auto">
                {timestamp.toLocaleTimeString()}
              </span>
            </div>

            {/* Tool Call Display */}
            {isToolCall && toolCall && (
              <div className="rounded-lg bg-cyan-500/5 border border-cyan-500/20 p-4 space-y-2">
                <div className="flex items-center gap-2 text-cyan-400">
                  <Wrench className="h-4 w-4" />
                  <code className="text-sm font-mono font-semibold">{toolCall.name}</code>
                </div>
                {Object.keys(toolCall.args).length > 0 && (
                  <pre className="text-xs bg-black/30 text-cyan-100 p-3 rounded-md overflow-x-auto border border-cyan-500/10">
                    <code>{JSON.stringify(toolCall.args, null, 2)}</code>
                  </pre>
                )}
              </div>
            )}

            {/* Tool Result Display */}
            {isToolResult && (
              <div className="rounded-lg bg-green-500/5 border border-green-500/20 p-4 space-y-2">
                <div className="flex items-center gap-2 text-green-400 font-medium text-sm">
                  <CheckCircle2 className="h-4 w-4" />
                  <span>Tool Result</span>
                </div>
                <pre className="text-xs bg-black/30 text-green-100 p-3 rounded-md overflow-x-auto whitespace-pre-wrap border border-green-500/10">
                  {content.length > 1000 ? content.slice(0, 1000) + '\n\n... (truncated)' : content}
                </pre>
              </div>
            )}

            {/* Error Display */}
            {isError && (
              <div className="rounded-lg bg-red-500/5 border border-red-500/20 p-4">
                <div className="flex items-start gap-2 text-red-400">
                  <AlertCircle className="h-4 w-4 mt-0.5 flex-shrink-0" />
                  <p className="text-sm">{content}</p>
                </div>
              </div>
            )}

            {/* Main Text Content (Markdown) - with ReAct pattern parsing */}
            {!isToolCall && !isToolResult && !isError && (
              <RenderContent content={content} isThinking={isThinking} />
            )}

            {/* Message Actions */}
            <MessageActions
              content={content}
              isStreaming={isStreaming}
              onCancel={onCancel}
              onRetry={onRetry}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

interface MessageListProps {
  messages: ChatMessage[];
  isLoading?: boolean;
  onCancelMessage?: (messageId: string) => void;
  onRetryMessage?: (messageId: string) => void;
}

export function MessageList({ messages, isLoading, onCancelMessage, onRetryMessage }: MessageListProps) {
  return (
    <div className="divide-y divide-border/40">
      {messages.map((message) => (
        <MessageBubble
          key={message.id}
          message={message}
          onCancel={message.isStreaming ? () => onCancelMessage?.(message.id) : undefined}
          onRetry={!message.isStreaming && message.role === 'assistant' ? () => onRetryMessage?.(message.id) : undefined}
        />
      ))}

      {isLoading && !messages.some(m => m.isStreaming) && (
        <div className="py-6 px-4">
          <div className="max-w-3xl mx-auto">
            <div className="flex gap-4">
              <div className="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0">
                <Bot className="h-4 w-4 text-primary" />
              </div>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>Thinking...</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
