import ReactMarkdown from 'react-markdown';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import type { ConversationEntry } from './types';

interface ConversationHistoryProps {
  messages: ConversationEntry[];
  isLoading: boolean;
  onSuggestionClick: (suggestion: string) => void;
}

export function ConversationHistory({
  messages,
  isLoading,
  onSuggestionClick,
}: ConversationHistoryProps) {
  return (
    <div className="flex-1 overflow-y-auto space-y-4 min-h-0">
      {messages.map((message, index) => (
        <div
          key={index}
          className={`flex ${message.type === 'user' ? 'justify-end' : 'justify-start'}`}
        >
          <div
            className={`max-w-xs lg:max-w-md px-4 py-2 rounded-lg ${
              message.type === 'user'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 text-gray-800'
            }`}
          >
            <div className="text-sm prose prose-sm max-w-none">
              {message.type === 'nora' ? (
                <ReactMarkdown
                  components={{
                    p: ({children}) => <p className="mb-2 last:mb-0">{children}</p>,
                    strong: ({children}) => <strong className="font-semibold">{children}</strong>,
                    ul: ({children}) => <ul className="list-disc ml-4 mb-2">{children}</ul>,
                    ol: ({children}) => <ol className="list-decimal ml-4 mb-2">{children}</ol>,
                    li: ({children}) => <li className="mb-1">{children}</li>,
                  }}
                >
                  {message.content}
                </ReactMarkdown>
              ) : (
                message.content
              )}
            </div>
            {message.response && (
              <div className="mt-2 space-y-1">
                {/* Executive Actions */}
                {message.response.actions.length > 0 && (
                  <div className="text-xs space-y-1">
                    {message.response.actions.map(action => (
                      <Badge
                        key={action.actionId}
                        variant="secondary"
                        className="text-xs"
                      >
                        {action.actionType}: {action.description}
                      </Badge>
                    ))}
                  </div>
                )}
                {/* Follow-up Suggestions */}
                {message.response.followUpSuggestions.length > 0 && (
                  <div className="text-xs text-gray-600">
                    <span className="font-medium">Suggestions:</span>
                    <ul className="list-disc list-inside ml-2">
                      {message.response.followUpSuggestions.map((suggestion, i) => (
                        <li key={i} className="cursor-pointer hover:text-blue-600"
                            onClick={() => onSuggestionClick(suggestion)}>
                          {suggestion}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            )}
            <div className="text-xs opacity-50 mt-1">
              {message.timestamp.toLocaleTimeString()}
            </div>
          </div>
        </div>
      ))}
      {isLoading && (
        <div className="flex justify-start">
          <div className="bg-gray-100 rounded-lg px-4 py-2">
            <Loader message="Nora is thinking..." size={16} />
          </div>
        </div>
      )}
    </div>
  );
}
