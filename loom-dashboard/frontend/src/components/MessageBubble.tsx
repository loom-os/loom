import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import rehypeRaw from 'rehype-raw';
import remarkGfm from 'remark-gfm';
import { Bot, User, Brain, Wrench, CheckCircle2, AlertCircle, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ChatMessage, ContentType } from '@/hooks/useChat';
import 'highlight.js/styles/github-dark.css';

interface MessageBubbleProps {
  message: ChatMessage;
}

const ContentTypeIcon: Record<ContentType, React.ReactNode> = {
  text: null,
  thinking: <Brain className="h-3 w-3" />,
  tool_call: <Wrench className="h-3 w-3" />,
  tool_result: <CheckCircle2 className="h-3 w-3" />,
  error: <AlertCircle className="h-3 w-3" />,
};

const ContentTypeLabel: Record<ContentType, string> = {
  text: '',
  thinking: 'Thinking',
  tool_call: 'Tool Call',
  tool_result: 'Tool Result',
  error: 'Error',
};

const ContentTypeStyle: Record<ContentType, string> = {
  text: '',
  thinking: 'text-purple-400 bg-purple-500/10 border-purple-500/30',
  tool_call: 'text-cyan-400 bg-cyan-500/10 border-cyan-500/30',
  tool_result: 'text-green-400 bg-green-500/10 border-green-500/30',
  error: 'text-red-400 bg-red-500/10 border-red-500/30',
};

export function MessageBubble({ message }: MessageBubbleProps) {
  const { role, content, timestamp, agentId, agentName, contentType = 'text', isStreaming, toolCall } = message;

  if (role === 'system') {
    return (
      <div className="flex justify-center my-4">
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

  return (
    <div
      className={cn(
        'flex gap-3',
        isUser ? 'justify-end' : 'justify-start'
      )}
    >
      {!isUser && (
        <div className="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0 mt-1">
          <Bot className="h-4 w-4 text-primary" />
        </div>
      )}

      <div
        className={cn(
          'max-w-[75%] rounded-2xl',
          isUser
            ? 'bg-primary text-primary-foreground px-4 py-3'
            : cn(
                'bg-muted/50 text-foreground',
                contentType !== 'text' && 'border',
                ContentTypeStyle[contentType],
                isThinking && 'italic',
                isError && 'border-red-500/50'
              )
        )}
      >
        {/* Agent & Content Type Header */}
        {!isUser && (agentId || agentName || contentType !== 'text') && (
          <div className="flex items-center gap-2 mb-2 px-4 pt-3">
            {(agentId || agentName) && (
              <div className="text-xs font-medium text-primary">
                {agentName || agentId}
              </div>
            )}

            {contentType !== 'text' && (
              <div className={cn(
                'flex items-center gap-1 text-xs px-2 py-0.5 rounded-full border',
                ContentTypeStyle[contentType]
              )}>
                {ContentTypeIcon[contentType]}
                <span>{ContentTypeLabel[contentType]}</span>
              </div>
            )}

            {isStreaming && (
              <div className="flex gap-0.5 ml-auto">
                <div className="w-1 h-1 rounded-full bg-primary animate-pulse" style={{ animationDelay: '0ms' }}></div>
                <div className="w-1 h-1 rounded-full bg-primary animate-pulse" style={{ animationDelay: '150ms' }}></div>
                <div className="w-1 h-1 rounded-full bg-primary animate-pulse" style={{ animationDelay: '300ms' }}></div>
              </div>
            )}
          </div>
        )}

        {/* Tool Call Display */}
        {isToolCall && toolCall && (
          <div className="space-y-2 px-4 pb-3">
            <div className="flex items-center gap-2">
              <Wrench className="h-4 w-4 text-cyan-400" />
              <code className="text-sm font-mono text-cyan-300">{toolCall.name}</code>
            </div>
            {Object.keys(toolCall.args).length > 0 && (
              <pre className="text-xs bg-black/20 p-2 rounded overflow-x-auto">
                <code>{JSON.stringify(toolCall.args, null, 2)}</code>
              </pre>
            )}
          </div>
        )}

        {/* Main Content */}
        {!isToolCall && (
          <div className={cn(
            'prose prose-sm prose-invert max-w-none',
            isUser ? 'px-0 py-0' : 'px-4 pb-2',
            isThinking && 'prose-p:italic prose-p:text-purple-300'
          )}>
            {isUser ? (
              <p className="whitespace-pre-wrap text-sm m-0">{content}</p>
            ) : isError ? (
              <p className="text-sm text-red-400 m-0">❌ {content}</p>
            ) : isToolResult ? (
              <div className="text-sm">
                <p className="text-green-400 font-medium mb-1">✅ Result:</p>
                <pre className="text-xs bg-black/20 p-2 rounded overflow-x-auto whitespace-pre-wrap">
                  {content.length > 500 ? content.slice(0, 500) + '...' : content}
                </pre>
              </div>
            ) : (
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                rehypePlugins={[rehypeHighlight, rehypeRaw]}
                components={{
                  // Customize code blocks
                  code({ className, children, ...props }: any) {
                    const match = /language-(\w+)/.exec(className || '');
                    const isInline = !className;
                    return !isInline ? (
                      <code className={className} {...props}>
                        {children}
                      </code>
                    ) : (
                      <code className="bg-black/30 px-1 py-0.5 rounded text-sm" {...props}>
                        {children}
                      </code>
                    );
                  },
                  // Customize paragraphs
                  p({ children }) {
                    return <p className="mb-2 last:mb-0">{children}</p>;
                  },
                  // Customize links
                  a({ children, href }) {
                    return (
                      <a
                        href={href}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-primary hover:underline"
                      >
                        {children}
                      </a>
                    );
                  },
                }}
              >
                {content}
              </ReactMarkdown>
            )}
          </div>
        )}

        {/* Timestamp */}
        <div className={cn(
          'text-[10px] mt-1.5 px-4 pb-3',
          isUser ? 'text-primary-foreground/70' : 'text-muted-foreground'
        )}>
          {timestamp.toLocaleTimeString()}
        </div>
      </div>

      {isUser && (
        <div className="w-8 h-8 rounded-full bg-secondary/10 flex items-center justify-center flex-shrink-0 mt-1">
          <User className="h-4 w-4 text-secondary-foreground" />
        </div>
      )}
    </div>
  );
}

interface MessageListProps {
  messages: ChatMessage[];
  isLoading?: boolean;
}

export function MessageList({ messages, isLoading }: MessageListProps) {
  return (
    <div className="space-y-4">
      {messages.map((message) => (
        <MessageBubble key={message.id} message={message} />
      ))}

      {isLoading && !messages.some(m => m.isStreaming) && (
        <div className="flex gap-3 justify-start">
          <div className="w-8 h-8 rounded-full bg-primary/10 flex items-center justify-center flex-shrink-0">
            <Bot className="h-4 w-4 text-primary" />
          </div>
          <div className="bg-muted/50 rounded-2xl px-4 py-3">
            <Loader2 className="h-4 w-4 animate-spin text-primary" />
          </div>
        </div>
      )}
    </div>
  );
}
